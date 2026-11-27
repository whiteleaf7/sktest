use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::parse::Value;
use crate::schema::{Key, Node, Schema, Type};

/// What the schema wants at a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    Type(Type),
    Table,
}

// `MissingKey` prints a bare `Type` ("bool"), and only a mismatch lists the accepted values.
impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expected::Type(Type::Bool) => f.write_str("bool (true or false)"),
            Expected::Type(ty) => write!(f, "{ty}"),
            Expected::Table => f.write_str("table"),
        }
    }
}

/// What the input has at a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    String(String),
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The key is not in the schema.
    UnknownKey { key: String },
    /// The key is required by the schema but not in the input.
    MissingKey { key: String, expected: Type },
    /// The value does not match the type, or a table and a value are mixed up.
    TypeMismatch {
        key: String,
        expected: Expected,
        found: Found,
    },
}

impl VerifyError {
    fn key(&self) -> &str {
        match self {
            VerifyError::UnknownKey { key }
            | VerifyError::MissingKey { key, .. }
            | VerifyError::TypeMismatch { key, .. } => key,
        }
    }
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::UnknownKey { key } => write!(f, "{key}: unknown key"),
            VerifyError::MissingKey { key, expected } => {
                write!(f, "{key}: missing required key (expected {expected})")
            }
            VerifyError::TypeMismatch {
                key,
                expected,
                found,
            } => {
                write!(f, "{key}: expected {expected}, got ")?;
                match (expected, found) {
                    (Expected::Table, Found::String(s)) => write!(f, "string {s:?}"),
                    (_, Found::String(s)) => write!(f, "{s:?}"),
                    (_, Found::Table) => f.write_str("table"),
                }
            }
        }
    }
}

/// Checks the parsed input against the schema and returns all errors, sorted by key.
pub fn verify(map: &HashMap<String, Value>, schema: &Schema) -> Result<(), Vec<VerifyError>> {
    let mut errors = Vec::new();
    verify_table(map, &schema.root, "", &mut errors);
    if errors.is_empty() {
        return Ok(());
    }
    errors.sort_by(|a, b| a.key().cmp(b.key()));
    Err(errors)
}

fn verify_table(
    map: &HashMap<String, Value>,
    schema: &BTreeMap<Key, Node>,
    prefix: &str,
    errors: &mut Vec<VerifyError>,
) {
    for (key, node) in schema {
        let path = join(prefix, key.as_str());
        match (node, map.get(key.as_str())) {
            (node, None) => missing(node, path, errors),
            (Node::Field(field), Some(Value::String(s))) => {
                if !accepts(field.ty, s) {
                    errors.push(VerifyError::TypeMismatch {
                        key: path,
                        expected: Expected::Type(field.ty),
                        found: Found::String(s.clone()),
                    });
                }
            }
            (Node::Field(field), Some(Value::Map(_))) => errors.push(VerifyError::TypeMismatch {
                key: path,
                expected: Expected::Type(field.ty),
                found: Found::Table,
            }),
            (Node::Table(child), Some(Value::Map(value))) => {
                verify_table(value, child, &path, errors);
            }
            (Node::Table(_), Some(Value::String(s))) => errors.push(VerifyError::TypeMismatch {
                key: path,
                expected: Expected::Table,
                found: Found::String(s.clone()),
            }),
        }
    }

    for (key, value) in map {
        if !schema.contains_key(key.as_str()) {
            unknown_leaves(value, &join(prefix, key), errors);
        }
    }
}

/// Reports a key that is not in the input. For a table, reports every required key in it,
/// so each error matches one line to add.
fn missing(node: &Node, path: String, errors: &mut Vec<VerifyError>) {
    match node {
        Node::Field(field) => {
            if !field.optional {
                errors.push(VerifyError::MissingKey {
                    key: path,
                    expected: field.ty,
                });
            }
        }
        Node::Table(child) => {
            for (key, node) in child {
                missing(node, join(&path, key.as_str()), errors);
            }
        }
    }
}

/// Reports every key under an unknown key, so each error matches one line of the input.
fn unknown_leaves(value: &Value, path: &str, errors: &mut Vec<VerifyError>) {
    match value {
        Value::String(_) => errors.push(VerifyError::UnknownKey {
            key: path.to_string(),
        }),
        Value::Map(map) => {
            for (key, child) in map {
                unknown_leaves(child, &join(path, key), errors);
            }
        }
    }
}

fn accepts(ty: Type, s: &str) -> bool {
    match ty {
        Type::String => true,
        Type::Bool => s.parse::<bool>().is_ok(),
        Type::Integer => s.parse::<i64>().is_ok(),
    }
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    const SCHEMA: &str = "\
endpoint = \"string\"
empty = \"string\"
debug = \"bool\"
retry = \"integer\"
note = \"string?\"
log.file = \"string\"
log.level = \"string?\"
log.rotate.size = \"integer\"
db.host = \"string\"
db.port = \"integer?\"
";

    fn check(input: &str) -> Result<(), Vec<String>> {
        let schema = Schema::from_toml(SCHEMA).unwrap();
        let map = parse(input).unwrap();
        verify(&map, &schema).map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn accepts_valid_input() {
        let inputs = [
            // optional keys are left out
            "\
endpoint = localhost:3000
empty =
debug = true
retry = 3
log.file = /var/log/app.log
log.rotate.size = 10
db.host = localhost
",
            // optional keys are written, and other accepted values
            "\
endpoint = 1
empty =
debug = false
retry = -1
note = true
log.file = x
log.level = info
log.rotate.size = +007
db.host = h
db.port = 9223372036854775807
",
        ];

        for input in inputs {
            assert_eq!(check(input), Ok(()), "input: {input:?}");
        }
    }

    #[test]
    fn reports_all_errors_sorted_by_key() {
        let input = "\
endpoint = localhost
debug = True
retry = 1_000
retyr = 3
log = abc
db.host.name = h
db.port = 9223372036854775808
extra.a = 1
extra.b.c = 2
";

        let expected = [
            "db.host: expected string, got table",
            "db.port: expected integer, got \"9223372036854775808\"",
            "debug: expected bool (true or false), got \"True\"",
            "empty: missing required key (expected string)",
            "extra.a: unknown key",
            "extra.b.c: unknown key",
            "log: expected table, got string \"abc\"",
            "retry: expected integer, got \"1_000\"",
            "retyr: unknown key",
        ];

        assert_eq!(check(input), Err(expected.map(String::from).to_vec()));

        // a whole table that is missing reports each required key in it
        let expected = [
            "db.host: missing required key (expected string)",
            "debug: missing required key (expected bool)",
            "empty: missing required key (expected string)",
            "endpoint: missing required key (expected string)",
            "log.file: missing required key (expected string)",
            "log.rotate.size: missing required key (expected integer)",
            "retry: missing required key (expected integer)",
        ];

        assert_eq!(check(""), Err(expected.map(String::from).to_vec()));
    }
}
