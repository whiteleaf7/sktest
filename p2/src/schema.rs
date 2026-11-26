use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};

/// A value type that a key accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    String,
    Bool,
    Integer,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Type::String => "string",
            Type::Bool => "bool",
            Type::Integer => "integer",
        })
    }
}

/// A key with its type. `"bool?"` makes an optional key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub ty: Type,
    pub optional: bool,
}

impl TryFrom<&str> for Field {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, String> {
        let (name, optional) = match s.strip_suffix('?') {
            Some(name) => (name, true),
            None => (s, false),
        };
        let ty = match name {
            "string" => Type::String,
            "bool" => Type::Bool,
            "integer" => Type::Integer,
            _ => {
                return Err(format!(
                    "unknown type \"{s}\" (expected string, bool or integer, optionally followed by '?')"
                ));
            }
        };
        Ok(Field { ty, optional })
    }
}

/// A key of the schema. It must be a key that `parse` can produce:
/// not empty, no `.`, and no spaces around it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct Key(pub String);

impl TryFrom<String> for Key {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        if s.is_empty() {
            return Err("empty key".to_string());
        }
        if s.contains('.') {
            return Err(format!(
                "key \"{s}\" contains '.' (write nested keys as log.file, not \"log.file\")"
            ));
        }
        if s.trim() != s {
            return Err(format!("key \"{s}\" has spaces around it"));
        }
        Ok(Key(s))
    }
}

/// A node of the schema: a type for a key, or a table of nested keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Field(Field),
    Table(BTreeMap<Key, Node>),
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NodeVisitor)
    }
}

struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a type name such as \"string\" or \"bool?\", or a table")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Node, E> {
        Field::try_from(v).map(Node::Field).map_err(E::custom)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut table = BTreeMap::new();
        while let Some((key, node)) = map.next_entry::<Key, Node>()? {
            table.insert(key, node);
        }
        // `parse` never produces an empty map, so an empty table can never match.
        if table.is_empty() {
            return Err(de::Error::custom("empty table"));
        }
        Ok(Node::Table(table))
    }
}

/// The schema loaded from a TOML file, such as:
///
/// ```toml
/// endpoint = "string"
/// debug = "bool?"
/// log.file = "string"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Schema {
    pub root: BTreeMap<Key, Node>,
}

/// The schema file is not valid. The message has the line and column of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for SchemaError {}

impl Schema {
    pub fn from_toml(input: &str) -> Result<Schema, SchemaError> {
        toml::from_str(input).map_err(|e| {
            let offset = e.span().map_or(0, |span| span.start);
            let before = &input[..offset];
            let line = before.matches('\n').count() + 1;
            let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            SchemaError {
                line,
                column,
                message: e.message().trim().to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        Key(s.to_string())
    }

    fn field(ty: Type, optional: bool) -> Node {
        Node::Field(Field { ty, optional })
    }

    fn table<const N: usize>(pairs: [(&str, Node); N]) -> BTreeMap<Key, Node> {
        pairs.into_iter().map(|(k, v)| (key(k), v)).collect()
    }

    #[test]
    fn loads_all_forms() {
        let input = "\
# comment
endpoint = \"string\"
debug = \"bool?\"
retry = \"integer\"
log.file = \"string\"
log . level = \"string?\"
\"キー\" = \"string\"
-double = \"string\"

[net.ipv4]
ip_forward = \"bool\"
";

        let expected = table([
            ("endpoint", field(Type::String, false)),
            ("debug", field(Type::Bool, true)),
            ("retry", field(Type::Integer, false)),
            (
                "log",
                Node::Table(table([
                    ("file", field(Type::String, false)),
                    ("level", field(Type::String, true)),
                ])),
            ),
            ("キー", field(Type::String, false)),
            ("-double", field(Type::String, false)),
            (
                "net",
                Node::Table(table([(
                    "ipv4",
                    Node::Table(table([("ip_forward", field(Type::Bool, false))])),
                )])),
            ),
        ]);

        assert_eq!(Schema::from_toml(input), Ok(Schema { root: expected }));
        assert_eq!(
            Schema::from_toml(""),
            Ok(Schema {
                root: BTreeMap::new()
            })
        );
    }

    #[test]
    fn reports_errors() {
        // (input, line, column, part of the message)
        let cases = [
            ("a = \"intger\"", 1, 5, "unknown type \"intger\""),
            ("a = \"?\"", 1, 5, "unknown type \"?\""),
            ("a = \"bool??\"", 1, 5, "unknown type \"bool??\""),
            ("a = \"Bool\"", 1, 5, "unknown type \"Bool\""),
            ("ok = \"string\"\na = 1", 2, 5, "invalid type: integer `1`"),
            ("a = true", 1, 5, "invalid type: boolean `true`"),
            ("a = [\"string\"]", 1, 5, "invalid type: sequence"),
            ("\"a.b\" = \"string\"", 1, 1, "key \"a.b\" contains '.'"),
            ("x.\"a.b\" = \"string\"", 1, 3, "key \"a.b\" contains '.'"),
            ("\"\" = \"string\"", 1, 1, "empty key"),
            (
                "\" a\" = \"string\"",
                1,
                1,
                "key \" a\" has spaces around it",
            ),
            ("[log]\n", 1, 1, "empty table"),
            ("a = {}", 1, 5, "empty table"),
            ("a = \"string\"\na = \"bool\"", 2, 1, "duplicate key"),
            ("a = ", 1, 5, ""),
        ];

        for (input, line, column, message) in cases {
            let error = Schema::from_toml(input).expect_err(input);
            assert_eq!(
                (error.line, error.column),
                (line, column),
                "input: {input:?}, error: {error}"
            );
            assert!(
                error.message.contains(message),
                "input: {input:?}, error: {error}"
            );
        }
    }
}
