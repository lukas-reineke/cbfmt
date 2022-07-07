use super::format::FormatError;
use super::tree;
use clap::Values;
use ignore::WalkBuilder;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

pub fn get_start_whitespace(text: &str) -> String {
    let mut result = String::new();

    for ch in text.chars() {
        if ch.is_whitespace() {
            result.push(ch)
        } else {
            break;
        }
    }

    result
}

pub fn get_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

pub fn get_files(files: Values) -> Result<Vec<String>, io::Error> {
    let mut result = Vec::new();

    for file in files {
        let meta = match fs::metadata(file) {
            Ok(m) => m,
            Err(e) => {
                return Err(io::Error::new(
                    e.kind(),
                    format!("{file}: {}", &e.to_string()),
                ))
            }
        };
        if meta.is_file() {
            result.push(file.to_string());
        } else {
            for entry in WalkBuilder::new(file)
                .hidden(false)
                .build()
                .filter_map(|e| e.ok())
            {
                let path = entry.path().display().to_string();
                let meta = fs::metadata(entry.path()).unwrap();
                if meta.is_file() && tree::get_parser_lang_from_filename(&path).is_some() {
                    result.push(path);
                }
            }
        }
    }
    result.sort();
    result.dedup();

    Ok(result)
}

pub fn get_parser(filename: Option<&str>, parser: Option<&str>) -> Result<String, FormatError> {
    if let Some(p) = parser {
        return Ok(p.to_owned());
    }
    if let Some(f) = filename {
        if let Some(p) = tree::get_parser_lang_from_filename(f) {
            return Ok(p.to_owned());
        }
    }
    Err(FormatError {
        msg: "Could not infer parser.".to_string(),
        filename: filename.map(|f| f.to_owned()),
        command: None,
        language: None,
        start: None,
    })
}

pub fn find_closest_config() -> Option<String> {
    let name = ".cbfmt.toml";
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

pub fn print_ok(stdout: &mut StandardStream, text: &str) {
    let mut color_spec = ColorSpec::new();
    print!("[");
    stdout
        .set_color(color_spec.set_fg(Some(Color::Green)).set_bold(true))
        .unwrap();
    print!("Okay");
    color_spec.clear();
    stdout.set_color(&color_spec).unwrap();
    println!("]: {text}");
}

pub fn print_unchanged(stdout: &mut StandardStream, text: &str) {
    let mut color_spec = ColorSpec::new();
    print!("[");
    stdout
        .set_color(color_spec.set_fg(Some(Color::Blue)).set_bold(true))
        .unwrap();
    print!("Same");
    color_spec.clear();
    stdout.set_color(&color_spec).unwrap();
    println!("]: {text}");
}

pub fn print_fail(stderr: &mut StandardStream, text: &str) {
    let mut color_spec = ColorSpec::new();
    eprint!("[");
    stderr
        .set_color(color_spec.set_fg(Some(Color::Yellow)).set_bold(true))
        .unwrap();
    eprint!("Fail");
    color_spec.clear();
    stderr.set_color(&color_spec).unwrap();
    eprintln!("]: {text}");
}

pub fn print_error(stderr: &mut StandardStream, text: &str) {
    let mut color_spec = ColorSpec::new();
    eprint!("[");
    stderr
        .set_color(color_spec.set_fg(Some(Color::Red)).set_bold(true))
        .unwrap();
    eprint!("Error");
    color_spec.clear();
    stderr.set_color(&color_spec).unwrap();
    eprintln!("]: {text}");
}
