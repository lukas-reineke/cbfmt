use super::config::Conf;
use super::tree;
use super::utils;
use futures::{stream::FuturesOrdered, StreamExt};
use std::fmt;
use std::io::{self, prelude::*, Write};
use std::process::{Command, Stdio};

#[derive(thiserror::Error, Debug)]
pub struct FormatError {
    pub msg: String,
    pub filename: Option<String>,
    pub command: Option<String>,
    pub language: Option<String>,
    pub start: Option<String>,
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        if let Some(filename) = &self.filename {
            write!(formatter, "{filename}")?;
        }
        if let Some(start) = &self.start {
            write!(formatter, "{start}")?;
        }
        if let Some(language) = &self.language {
            write!(formatter, " [{language}] ->")?;
        }
        if let Some(command) = &self.command {
            write!(formatter, " [{command}] ")?;
        }
        write!(formatter, "\n{}", self.msg)
    }
}

pub enum FormatResult {
    Ignored,
    Unchanged(String),
    Changed(String),
    Err(FormatError),
}

pub async fn run_file(
    conf: &Conf,
    filename: String,
    parser: Option<&str>,
    check: bool,
    write: bool,
) -> FormatResult {
    let parser = match parser {
        Some(p) => p,
        None => match tree::get_parser_lang_from_filename(&filename) {
            Some(p) => p,
            None => {
                return FormatResult::Ignored;
            }
        },
    };

    let file = match tokio::fs::read(&filename).await {
        Err(error) => {
            return FormatResult::Err(FormatError {
                msg: error.to_string(),
                filename: Some(filename),
                command: None,
                language: None,
                start: None,
            })
        }
        Ok(f) => f,
    };
    let mut buf = Vec::new();
    for line in file.lines() {
        let line = match line {
            Err(error) => {
                return FormatResult::Err(FormatError {
                    msg: error.to_string(),
                    filename: Some(filename),
                    command: None,
                    language: None,
                    start: None,
                })
            }
            Ok(l) => l,
        };
        buf.push(line);
    }

    match run(buf, conf, parser, check).await {
        FormatResult::Changed(r) => {
            if write {
                match tokio::fs::write(&filename, r).await {
                    Ok(_) => return FormatResult::Changed(filename),
                    Err(error) => {
                        return FormatResult::Err(FormatError {
                            msg: error.to_string(),
                            filename: Some(filename),
                            command: None,
                            language: None,
                            start: None,
                        })
                    }
                };
            }
            return FormatResult::Changed(filename);
        }
        FormatResult::Unchanged(_) => return FormatResult::Unchanged(filename),
        FormatResult::Err(mut error) => {
            error.filename = Some(filename);
            return FormatResult::Err(error);
        }
        FormatResult::Ignored => return FormatResult::Ignored,
    }
}

pub async fn run_stdin(conf: &Conf, parser: &str, check: bool) -> FormatResult {
    let buf = io::stdin().lines().map(|l| l.unwrap()).collect::<Vec<_>>();
    let buf_string = buf.join("\n");

    match run(buf, conf, parser, check).await {
        FormatResult::Changed(r) => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(r.as_bytes()).unwrap();
            return FormatResult::Changed("stdin".to_string());
        }
        FormatResult::Unchanged(r) => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(r.as_bytes()).unwrap();
            return FormatResult::Unchanged("stdin".to_string());
        }
        FormatResult::Err(e) => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(buf_string.as_bytes()).unwrap();
            return FormatResult::Err(e);
        }
        FormatResult::Ignored => return FormatResult::Ignored,
    };
}

struct FormatCtx {
    language: String,
    codeblock_start: usize,
    start: usize,
    end: usize,
    input_hash: u64,
}

async fn run(mut buf: Vec<String>, conf: &Conf, parser: &str, check: bool) -> FormatResult {
    let src = buf.join("\n");
    let src_bytes = src.as_bytes();
    let tree = match tree::get_tree(parser, &src_bytes) {
        Some(t) => t,
        None => panic!("No parser found for {}", parser),
    };
    let query = match tree::get_query(parser) {
        Some(q) => q,
        None => panic!("No query found for {}", parser),
    };

    let mut futures: FuturesOrdered<_> = FuturesOrdered::new();

    let mut cursor = tree_sitter::QueryCursor::new();
    for each_match in cursor.matches(&query, tree.root_node(), src_bytes) {
        let mut content = String::new();
        let mut ctx = FormatCtx {
            language: String::new(),
            codeblock_start: 0,
            start: 0,
            end: 0,
            input_hash: 0,
        };
        for capture in each_match.captures.iter() {
            let range = capture.node.range();
            let capture_name = &query.capture_names()[capture.index as usize];
            if capture_name == "language" {
                ctx.language = String::from(&src[range.start_byte..range.end_byte]);
            }
            if capture_name == "content" {
                ctx.start = range.start_point.row;
                ctx.end = range.end_point.row;
                let mut end_byte = range.end_byte;

                // Workaround for bug in markdown parser when the codeblock is the last thing in a
                // buffer
                if parser == "markdown" && &src[(end_byte - 3)..end_byte] == "```" {
                    end_byte -= 3
                }

                content = String::from(&src[range.start_byte..end_byte]);
            }
            if capture_name == "codeblock" {
                ctx.codeblock_start = range.start_point.row;
            }
        }

        let formatter = conf.languages.get(&ctx.language);
        let formatter = match formatter {
            Some(f) => f,
            None => continue,
        };
        let formatter = formatter.iter().map(|f| f.to_owned()).collect();

        ctx.input_hash = utils::get_hash(&content);
        futures.push(tokio::spawn(async move {
            format(ctx, formatter, &content).await
        }));
    }

    let mut formatted = false;
    let mut offset: i32 = 0;
    while let Some(output) = futures.next().await {
        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return FormatResult::Err(FormatError {
                    msg: e.to_string(),
                    filename: None,
                    command: None,
                    language: None,
                    start: None,
                });
            }
        };
        let (ctx, output) = match output {
            Ok(o) => o,
            Err(e) => return FormatResult::Err(e),
        };

        let start_row = &buf[(ctx.codeblock_start as i32 + offset) as usize];
        let whitespace = utils::get_start_whitespace(&start_row);

        let mut fixed_output = String::new();
        for line in output.lines() {
            fixed_output.push_str(&whitespace);
            fixed_output.push_str(&line);
            fixed_output.push_str("\n");
        }

        // trim start for the hash because treesitter ignores leading whitespace
        let output_hash = utils::get_hash(&fixed_output.trim_start());
        if ctx.input_hash != output_hash {
            formatted = true;
            if check {
                break;
            }
        }

        buf.drain((ctx.start as i32 + offset) as usize..(ctx.end as i32 + offset) as usize);

        let mut counter = 0;
        for (i, line) in fixed_output.lines().enumerate() {
            buf.insert(i + (ctx.start as i32 + offset) as usize, line.to_string());
            counter += 1;
        }

        offset = (counter - (ctx.end as i32 - ctx.start as i32)) + offset;
    }

    let output = buf.join("\n") + "\n";
    if formatted {
        return FormatResult::Changed(output);
    }
    return FormatResult::Unchanged(output);
}

async fn format(
    ctx: FormatCtx,
    formatter: Vec<String>,
    content: &str,
) -> Result<(FormatCtx, String), FormatError> {
    let mut result = String::from(content);
    let language = Some(ctx.language.to_owned());
    let start = Some(format!(":{}", ctx.start));

    for f in formatter.iter() {
        if f.len() == 0 {
            continue;
        }
        let f: Vec<_> = f.split_whitespace().collect();
        let command = f[0];
        let child = Command::new(command)
            .args(&f[1..])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                return Err(FormatError {
                    msg: e.to_string(),
                    filename: None,
                    command: Some(command.to_string()),
                    language,
                    start,
                });
            }
        };

        let stdin = match child.stdin.as_mut() {
            Some(s) => s,
            None => {
                return Err(FormatError {
                    msg: String::from("Child process stdin has not been captured"),
                    filename: None,
                    command: Some(command.to_string()),
                    language,
                    start,
                });
            }
        };
        match stdin.write_all(&result.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                return Err(FormatError {
                    msg: e.to_string(),
                    filename: None,
                    command: Some(command.to_string()),
                    language,
                    start,
                })
            }
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                return Err(FormatError {
                    msg: e.to_string(),
                    filename: None,
                    command: Some(command.to_string()),
                    language,
                    start,
                })
            }
        };

        if output.status.success() {
            result = String::from_utf8(output.stdout).unwrap();
        } else {
            return Err(FormatError {
                msg: String::from_utf8(output.stderr).unwrap(),
                filename: None,
                command: Some(command.to_string()),
                language,
                start,
            });
        }
    }

    return Ok((ctx, result));
}
