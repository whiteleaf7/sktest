use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A temporary file that is removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str, content: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!("p1-test-{}-{name}", std::process::id()));
        fs::write(&path, content).unwrap();
        TempFile(path)
    }

    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_p1"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn reads_file_and_prints_entries() {
    let file = TempFile::new("ok.conf", b"a=1\n");
    let schema = TempFile::new("ok.toml", b"a = \"integer\"\n");
    let output = run(&[file.path(), schema.path()]);

    assert!(output.status.success());
    assert_eq!(output.stdout, b"{\n    \"a\": \"1\",\n}\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn fails_without_two_arguments() {
    for args in [&[][..], &["a.conf"], &["a.conf", "a.toml", "extra"]] {
        let output = run(args);

        assert!(!output.status.success(), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        assert_eq!(
            output.stderr, b"usage: p1 <file> <schema>\n",
            "args: {args:?}"
        );
    }
}

#[test]
fn fails_when_file_is_missing() {
    let path = std::env::temp_dir().join(format!("p1-test-{}-missing.conf", std::process::id()));
    let schema = TempFile::new("missing.toml", b"a = \"integer\"\n");
    let output = run(&[path.to_str().unwrap(), schema.path()]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn reports_errors_with_the_path_to_fix() {
    let file = TempFile::new("bad.conf", b"retry = abc\nretyr = 3\n");

    // every schema violation is one line with the path of the input file
    let schema = TempFile::new("bad.toml", b"retry = \"integer\"\ndebug = \"bool\"\n");
    let output = run(&[file.path(), schema.path()]);

    let f = file.path();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "{f}: debug: missing required key (expected bool)\n\
             {f}: retry: expected integer, got \"abc\"\n\
             {f}: retyr: unknown key\n"
        )
    );

    // a broken schema is reported with the path of the schema file
    let schema = TempFile::new("broken.toml", b"retry = \"intger\"\n");
    let output = run(&[file.path(), schema.path()]);

    let s = schema.path();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "{s}: line 1, column 9: unknown type \"intger\" \
             (expected string, bool or integer, optionally followed by '?')\n"
        )
    );
}
