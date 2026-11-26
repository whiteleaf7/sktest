use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line has no `=`.
    MissingEquals { line: usize },
    /// The key is empty after trimming and removing `-`.
    EmptyKey { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingEquals { line } => write!(f, "line {line}: missing '='"),
            ParseError::EmptyKey { line } => write!(f, "line {line}: empty key"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<HashMap<String, String>, ParseError> {
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

        map.insert(key.to_string(), value.trim().to_string());
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse(""), Ok(HashMap::new()));
    }

    #[test]
    fn simple_assignment() {
        assert_eq!(parse("a=1"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn multiple_assignments() {
        let input = "a=1\nb=2\nnet.ipv4.ip_forward=1\n";
        assert_eq!(
            parse(input),
            Ok(map(&[("a", "1"), ("b", "2"), ("net.ipv4.ip_forward", "1")]))
        );
    }

    #[test]
    fn trims_key_and_value() {
        assert_eq!(parse("  a  =  1  "), Ok(map(&[("a", "1")])));
        assert_eq!(parse("\ta\t=\t1\t"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn keeps_inner_whitespace_of_value() {
        assert_eq!(parse("a = 1  2\t3"), Ok(map(&[("a", "1  2\t3")])));
    }

    #[test]
    fn splits_at_first_equals() {
        assert_eq!(parse("a=b=c"), Ok(map(&[("a", "b=c")])));
        assert_eq!(parse("a = = "), Ok(map(&[("a", "=")])));
    }

    #[test]
    fn empty_value_is_allowed() {
        assert_eq!(parse("a="), Ok(map(&[("a", "")])));
        assert_eq!(parse("a =   "), Ok(map(&[("a", "")])));
    }

    #[test]
    fn quotes_are_kept() {
        assert_eq!(parse(r#"a="1""#), Ok(map(&[("a", r#""1""#)])));
        assert_eq!(parse("a='x y'"), Ok(map(&[("a", "'x y'")])));
    }

    #[test]
    fn skips_blank_lines() {
        assert_eq!(parse("\n\n   \n\t\na=1\n\n"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn skips_comments() {
        let input = "# comment\n; comment\n  # indented\n\t; indented\na=1";
        assert_eq!(parse(input), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn comment_line_may_contain_equals() {
        assert_eq!(parse("# a=1\n; b=2"), Ok(HashMap::new()));
    }

    #[test]
    fn hash_after_value_is_part_of_value() {
        assert_eq!(parse("a=1 # note"), Ok(map(&[("a", "1 # note")])));
        assert_eq!(parse("a=1;2"), Ok(map(&[("a", "1;2")])));
    }

    #[test]
    fn removes_single_leading_dash() {
        assert_eq!(parse("-a=1"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn removes_only_one_dash() {
        assert_eq!(parse("--a=1"), Ok(map(&[("-a", "1")])));
        assert_eq!(parse("---a=1"), Ok(map(&[("--a", "1")])));
    }

    #[test]
    fn dash_after_leading_whitespace() {
        assert_eq!(parse("  -a=1"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn whitespace_between_dash_and_key() {
        assert_eq!(parse("- a=1"), Ok(map(&[("a", "1")])));
    }

    #[test]
    fn dash_inside_key_is_kept() {
        assert_eq!(parse("a-b=1"), Ok(map(&[("a-b", "1")])));
        assert_eq!(parse("a-=1"), Ok(map(&[("a-", "1")])));
    }

    #[test]
    fn dash_in_value_is_kept() {
        assert_eq!(parse("a=-1"), Ok(map(&[("a", "-1")])));
    }

    #[test]
    fn later_assignment_wins() {
        assert_eq!(parse("a=1\na=2"), Ok(map(&[("a", "2")])));
        assert_eq!(parse("a=1\n-a=2"), Ok(map(&[("a", "2")])));
        assert_eq!(parse("-a=1\na=2"), Ok(map(&[("a", "2")])));
    }

    #[test]
    fn dash_key_and_plain_key_are_different() {
        assert_eq!(parse("a=1\n--a=2"), Ok(map(&[("a", "1"), ("-a", "2")])));
    }

    #[test]
    fn handles_crlf() {
        assert_eq!(parse("a=1\r\nb=2\r\n"), Ok(map(&[("a", "1"), ("b", "2")])));
    }

    #[test]
    fn handles_unicode() {
        assert_eq!(parse("キー = 値"), Ok(map(&[("キー", "値")])));
    }

    #[test]
    fn error_missing_equals() {
        assert_eq!(parse("a"), Err(ParseError::MissingEquals { line: 1 }));
        assert_eq!(parse("-a"), Err(ParseError::MissingEquals { line: 1 }));
    }

    #[test]
    fn error_empty_key() {
        assert_eq!(parse("=1"), Err(ParseError::EmptyKey { line: 1 }));
        assert_eq!(parse("  = 1"), Err(ParseError::EmptyKey { line: 1 }));
        assert_eq!(parse("-=1"), Err(ParseError::EmptyKey { line: 1 }));
        assert_eq!(parse("- =1"), Err(ParseError::EmptyKey { line: 1 }));
    }

    #[test]
    fn error_reports_line_number() {
        let input = "# comment\n\na=1\nbad\nc=3";
        assert_eq!(parse(input), Err(ParseError::MissingEquals { line: 4 }));
    }

    #[test]
    fn error_display() {
        assert_eq!(
            ParseError::MissingEquals { line: 3 }.to_string(),
            "line 3: missing '='"
        );
        assert_eq!(
            ParseError::EmptyKey { line: 5 }.to_string(),
            "line 5: empty key"
        );
    }
}
