mod parse;
mod schema;
mod verify;

use std::collections::HashMap;
use std::env;
use std::fmt::Display;
use std::fs;
use std::process::ExitCode;

use parse::Value;
use schema::Schema;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [path, schema_path] = args.as_slice() else {
        eprintln!("usage: p1 <file> <schema>");
        return ExitCode::FAILURE;
    };

    match run(path, schema_path) {
        Ok(map) => {
            println!("{map:#?}");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Parses the file and checks it against the schema.
/// Each error is one line that starts with the path of the file to fix.
fn run(path: &str, schema_path: &str) -> Result<HashMap<String, Value>, Vec<String>> {
    let line = |path: &str, error: &dyn Display| format!("{path}: {error}");

    let text = fs::read_to_string(path).map_err(|e| vec![line(path, &e)])?;
    let map = parse::parse(&text).map_err(|e| vec![line(path, &e)])?;

    let schema_text = fs::read_to_string(schema_path).map_err(|e| vec![line(schema_path, &e)])?;
    let schema = Schema::from_toml(&schema_text).map_err(|e| vec![line(schema_path, &e)])?;

    verify::verify(&map, &schema)
        .map_err(|errors| errors.iter().map(|e| line(path, e)).collect::<Vec<_>>())?;

    Ok(map)
}
