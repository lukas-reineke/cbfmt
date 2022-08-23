use clap::{App, Arg, ArgMatches};
mod config;
mod format;
use format::FormatResult;
mod tree;
mod utils;
use futures::{stream::FuturesUnordered, StreamExt};
use std::process;
use termcolor::{ColorChoice, StandardStream};

#[tokio::main]
async fn main() {
    let (mut color_choice, clap_color_choice) = if atty::is(atty::Stream::Stdout) {
        (ColorChoice::Auto, clap::ColorChoice::Auto)
    } else {
        (ColorChoice::Never, clap::ColorChoice::Never)
    };

    let mut app =
        App::new("cbfmt")
            .version("0.1.4")
            .author("Lukas Reineke <lukas@reineke.jp>")
            .about("A tool to format codeblocks inside markdown and org documents.\nIt iterates over all codeblocks, and formats them with the tool(s) specified for the language of the block.")
            .arg(
                Arg::with_name("config")
                    .long("config")
                    .value_name("FILE")
                    .help("Sets a custom config file.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("check")
                    .short('c')
                    .long("check")
                    .takes_value(false)
                    .help("Check if the given files are formatted. Print the path to unformatted files and exit with exit code 1 if they are not.")
            )
            .arg(
                Arg::with_name("fail_fast")
                    .long("fail-fast")
                    .takes_value(false)
                    .help("Exit as soon as one file is not formatted correctly.")
            )
            .arg(
                Arg::with_name("write")
                    .short('w')
                    .long("write")
                    .takes_value(false)
                    .help("Edit files in-place.")
            )
            .arg(
                Arg::with_name("best_effort")
                    .long("best-effort")
                    .takes_value(false)
                    .help("Ignore formatting errors and continue with the next codeblock.")
            )
            .arg(
                Arg::with_name("parser")
                    .short('p')
                    .long("parser")
                    .value_name("markdown|org")
                    .help("Sets the parser to use.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("stdin_filepath")
                    .long("stdin-filepath")
                    .help("Path to the file to pretend that stdin comes from.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("color")
                    .long("color")
                    .value_name("never|auto|always")
                    .help("Use colored output.")
                    .default_value("auto")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("files")
                    .value_name("file/dir/glob")
                    .help("List of files to process. If no files are given cbfmt will read from Stdin.")
                    .index(1)
                    .multiple_values(true),
            )
            .color(clap_color_choice);

    let matches = app.to_owned().get_matches();

    if let Some(color) = matches.value_of("color") {
        if color == "never" {
            color_choice = ColorChoice::Never;
        } else if color == "always" {
            color_choice = ColorChoice::Always;
        }
    }

    if matches.values_of("files").is_none() && atty::is(atty::Stream::Stdin) {
        app.print_help().unwrap();
        return;
    }

    let mut stderr = StandardStream::stderr(color_choice);

    let config_path = match matches.value_of("config") {
        Some(p) => p.to_owned(),
        None => match utils::find_closest_config() {
            Some(p) => p,
            None => {
                utils::print_error(&mut stderr, "Could not find config file.");
                process::exit(1);
            }
        },
    };
    let conf = match config::get(&config_path) {
        Ok(c) => c,
        Err(_) => {
            utils::print_error(&mut stderr, "Could not parse config file.");
            process::exit(1);
        }
    };

    match matches.values_of("files") {
        Some(_) => use_files(matches, &conf, color_choice).await,
        None => use_stdin(matches, &conf).await,
    }
}

async fn use_files(matches: ArgMatches, conf: &config::Conf, color_choice: ColorChoice) {
    let mut stdout = StandardStream::stdout(color_choice);
    let mut stderr = StandardStream::stderr(color_choice);

    let check = matches.is_present("check");
    let write = matches.is_present("write");
    let best_effort = matches.is_present("best_effort");
    let fail_fast = matches.is_present("fail_fast");
    let files = matches.values_of("files").unwrap();
    let parser = matches.value_of("parser");

    let mut futures: FuturesUnordered<_> = FuturesUnordered::new();
    let files = match utils::get_files(files) {
        Ok(f) => f,
        Err(e) => {
            utils::print_error(&mut stderr, &e.to_string());
            process::exit(1);
        }
    };
    for filename in files {
        futures.push(format::run_file(conf, filename, parser, write, best_effort));
    }

    let mut error_count = 0;
    let mut unchanged_count = 0;
    let mut changed_count = 0;

    while let Some(result) = futures.next().await {
        match result {
            FormatResult::Unchanged(f) => {
                unchanged_count += 1;
                if check {
                    continue;
                }
                if write {
                    utils::print_unchanged(&mut stdout, &f);
                } else {
                    utils::print_ok(&mut stdout, &f);
                }
            }
            FormatResult::Changed(f) => {
                changed_count += 1;
                if check {
                    eprintln!("{f}")
                } else if write {
                    utils::print_ok(&mut stdout, &f);
                } else {
                    utils::print_fail(&mut stderr, &f);
                }
                if !write && fail_fast {
                    println!("Failed fast...");
                    break;
                }
            }
            FormatResult::Err(e) => {
                error_count += 1;
                if check {
                    let filename = match &e.filename {
                        Some(f) => f,
                        None => "Unknown",
                    };
                    eprintln!("{filename}");
                } else {
                    utils::print_error(&mut stderr, &e.to_string());
                }
                if fail_fast {
                    println!("Failed fast...");
                    break;
                }
            }
        }
    }

    let total_count = unchanged_count + changed_count + error_count;
    if write {
        println!("\n[{changed_count}/{total_count}] files were written.");
    }

    if !write && !check {
        println!("\n[{unchanged_count}/{total_count}] files are formatted correctly.");
    }

    if error_count > 0 || (changed_count > 0 && !write) {
        process::exit(1);
    }
}

async fn use_stdin(matches: ArgMatches, conf: &config::Conf) {
    let parser = matches.value_of("parser");
    let filename = matches.value_of("stdin_filepath");
    let best_effort = matches.is_present("best_effort");

    if let FormatResult::Err(e) = format::run_stdin(conf, filename, parser, best_effort).await {
        eprintln!("{e}");
        process::exit(1);
    }
}
