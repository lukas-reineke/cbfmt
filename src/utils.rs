use clap::Values;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

pub fn get_start_whitespace(text: &str) -> String {
    let mut result = String::new();

    for ch in text.chars() {
        if ch.is_whitespace() {
            result.push(ch)
        } else {
            break;
        }
    }

    return result;
}

pub fn get_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    return hasher.finish();
}

pub fn get_files(files: Values) -> Vec<String> {
    let mut result = Vec::new();

    for file in files {
        for entry in WalkDir::new(&file).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path().display().to_string();
            let meta = match fs::metadata(&path) {
                Err(e) => {
                    eprintln!("Can't open '{path}': {e}");
                    continue;
                }
                Ok(m) => m,
            };

            if meta.is_file() {
                result.push(path);
            }
        }
    }
    result.sort();
    result.dedup();

    return result;
}

pub fn find_closest_config() -> Option<String> {
    let name = "codeblock-format.toml";
    let mut current_dir = match env::current_dir() {
        Ok(c) => c,
        Err(_) => return None,
    };
    loop {
        let path = current_dir.join(name);

        if path.exists() {
            return Some(path.to_str()?.to_string());
        }
        match current_dir.parent() {
            Some(p) => current_dir = p.to_path_buf(),
            None => return None,
        }
    }
}

pub fn print_ok(stdout: &mut StandardStream) {
    print!("[");
    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .unwrap();
    print!("OK");
    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::White)))
        .unwrap();
    print!("]: ");
}

pub fn print_fail(stderr: &mut StandardStream) {
    eprint!("[");
    stderr
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
        .unwrap();
    eprint!("Fail");
    stderr
        .set_color(ColorSpec::new().set_fg(Some(Color::White)))
        .unwrap();
    eprint!("]: ");
}

pub fn print_error(stderr: &mut StandardStream) {
    eprint!("[");
    stderr
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .unwrap();
    eprint!("Error");
    stderr
        .set_color(ColorSpec::new().set_fg(Some(Color::White)))
        .unwrap();
    eprint!("]: ");
}
