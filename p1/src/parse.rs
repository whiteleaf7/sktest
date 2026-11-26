use std::collections::HashMap;
use std::fmt;

/// A parsed value. A key with `.` such as `log.file` becomes nested maps.
#[derive(Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Map(HashMap<String, Value>),
}

// Print the inner value only, so the output looks like plain nested maps.
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => fmt::Debug::fmt(s, f),
            Value::Map(m) => fmt::Debug::fmt(m, f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line has no `=`.
    MissingEquals { line: usize },
    /// The key is empty after trimming and removing `-`.
    EmptyKey { line: usize },
    /// A part of the key between `.` is empty, such as `a..b` or `a.`.
    EmptyKeySegment { line: usize },
    /// The key is used both as a value and as a parent, such as `a=1` and `a.b=2`.
    KeyConflict { line: usize, key: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingEquals { line } => write!(f, "line {line}: missing '='"),
            ParseError::EmptyKey { line } => write!(f, "line {line}: empty key"),
            ParseError::EmptyKeySegment { line } => {
                write!(f, "line {line}: empty part in key")
            }
            ParseError::KeyConflict { line, key } => {
                write!(f, "line {line}: key '{key}' conflicts with an existing key")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<HashMap<String, Value>, ParseError> {
    let mut map = HashMap::new();

    for (index, raw) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or(ParseError::MissingEquals { line: line_number })?;

        // NOTE: parser として key の存在有無でエラーになる状況にはならないため、- は単純に正常な
        // 場合のパターンとして適用する
        // ex.) -a=1 -> { a: 1 }, --a=1 -> { "-a": 1 }
        let key = key.strip_prefix('-').unwrap_or(key).trim();
        if key.is_empty() {
            return Err(ParseError::EmptyKey { line: line_number });
        }

        let segments: Vec<&str> = key.split('.').map(str::trim).collect();
        if segments.iter().any(|s| s.is_empty()) {
            return Err(ParseError::EmptyKeySegment { line: line_number });
        }

        insert(&mut map, &segments, value.trim().to_string()).map_err(|()| {
            ParseError::KeyConflict {
                line: line_number,
                key: key.to_string(),
            }
        })?;
    }

    Ok(map)
}

/// Inserts `value` at the path `segments`, creating parent maps as needed.
/// Returns `Err(())` when a parent is already a string, or the target is already a map.
fn insert(map: &mut HashMap<String, Value>, segments: &[&str], value: String) -> Result<(), ()> {
    let (last, parents) = segments.split_last().expect("segments is not empty");

    let mut current = map;
    for segment in parents {
        let entry = current
            .entry(segment.to_string())
            .or_insert_with(|| Value::Map(HashMap::new()));
        current = match entry {
            Value::Map(child) => child,
            Value::String(_) => return Err(()),
        };
    }

    if let Some(Value::Map(_)) = current.get(*last) {
        return Err(());
    }
    current.insert(last.to_string(), Value::String(value));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(value: &str) -> Value {
        Value::String(value.to_string())
    }

    fn m<const N: usize>(pairs: [(&str, Value); N]) -> HashMap<String, Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn parses_all_rules() {
        let input = "\
# comment
; comment
  # indented comment

endpoint = localhost:3000
  spaced  =  a  b  
empty =
quoted = \"x\"
equals = a=b
hash = 1 # not a comment
-dash = 1
--double = 1
dup = 1
dup = 2
version = 1.2.3
log.file = /var/log/console.log
log . level = info
log.rotate.size = 10
log.rotate.count = 5
-net.ipv4.ip_forward = 1
キー = 値
crlf = 1\r
";

        let expected = m([
            ("endpoint", s("localhost:3000")),
            ("spaced", s("a  b")),
            ("empty", s("")),
            ("quoted", s("\"x\"")),
            ("equals", s("a=b")),
            ("hash", s("1 # not a comment")),
            ("dash", s("1")),
            ("-double", s("1")),
            ("dup", s("2")),
            ("version", s("1.2.3")),
            (
                "log",
                Value::Map(m([
                    ("file", s("/var/log/console.log")),
                    ("level", s("info")),
                    (
                        "rotate",
                        Value::Map(m([("size", s("10")), ("count", s("5"))])),
                    ),
                ])),
            ),
            (
                "net",
                Value::Map(m([("ipv4", Value::Map(m([("ip_forward", s("1"))])))])),
            ),
            ("キー", s("値")),
            ("crlf", s("1")),
        ]);

        assert_eq!(parse(input), Ok(expected));
    }

    #[test]
    fn reports_errors() {
        let conflict = |line, key: &str| ParseError::KeyConflict {
            line,
            key: key.to_string(),
        };
        let cases = [
            ("a", ParseError::MissingEquals { line: 1 }),
            (
                "# comment\n\na=1\nbad",
                ParseError::MissingEquals { line: 4 },
            ),
            ("=1", ParseError::EmptyKey { line: 1 }),
            ("-=1", ParseError::EmptyKey { line: 1 }),
            ("a..b=1", ParseError::EmptyKeySegment { line: 1 }),
            ("a.=1", ParseError::EmptyKeySegment { line: 1 }),
            ("a=1\na.b=2", conflict(2, "a.b")),
            ("a.b=1\na=2", conflict(2, "a")),
            ("log.file=x\nlog.file.a=1", conflict(2, "log.file.a")),
            ("log.file.a=1\nlog.file=x", conflict(2, "log.file")),
        ];

        for (input, expected) in cases {
            assert_eq!(parse(input), Err(expected), "input: {input:?}");
        }
    }
}
