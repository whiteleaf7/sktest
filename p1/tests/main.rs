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
    let output = run(&[file.path()]);

    assert!(output.status.success());
    assert_eq!(output.stdout, b"{\n    \"a\": \"1\",\n}\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn fails_without_arguments() {
    let output = run(&[]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn fails_when_file_is_missing() {
    let path = std::env::temp_dir().join(format!("p1-test-{}-missing.conf", std::process::id()));
    let output = run(&[path.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
