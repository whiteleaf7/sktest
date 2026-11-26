mod parse;

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: p1 <file>");
        return ExitCode::FAILURE;
    };

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let map = match parse::parse(&text) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut entries: Vec<_> = map.iter().collect();
    entries.sort();
    for (key, value) in entries {
        println!("{key} = {value}");
    }

    ExitCode::SUCCESS
}
