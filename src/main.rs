use clap::{App, Arg, ArgMatches};
mod config;
mod process;
mod tree;
mod utils;
use futures::{stream::FuturesUnordered, StreamExt};
use termcolor::{ColorChoice, StandardStream};




#[tokio::main]
async fn main() {
    let matches =
        App::new("cbfmt")
            .version("0.1.0")
            .author("Lukas Reineke <lukas@reineke.jp>")
            .about("A tool to format codeblocks inside markdown and org documents.\nIt iterates over all codeblocks, and formats them with the tool specified for that language.")
            .arg(
                Arg::with_name("config")
                    .long("config")
                    .value_name("FILE")
                    .help("Sets a custom config file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("check")
                    .short('c')
                    .long("check")
                    .takes_value(false)
                    .help("Check if the given files are formatted. Print the path to unformatted files and exit with exit code 0 if they are not.")
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
                Arg::with_name("parser")
                    .short('p')
                    .long("parser")
                    .value_name("markdown|org")
                    .help("Sets the parser to use. Required for Stdin.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("files")
                    .value_name("file/dir/glob")
                    .help("List of files to process. If no files are given cbfmt will read from Stdin.")
                    .index(1)
                    .multiple_values(true),
            )
            .get_matches();

    let color_choice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stderr = StandardStream::stderr(color_choice);

    let config_path = match matches.value_of("config") {
        Some(p) => p.to_owned(),
        None => match utils::find_closest_config() {
            Some(p) => p,
            None => {
                utils::print_error(&mut stderr);
                eprintln!("Could not find config file");
                std::process::exit(1);
            }
        },
    };
    let conf = match config::get(&config_path) {
        Ok(c) => c,
        Err(_) => {
            utils::print_error(&mut stderr);
            eprintln!("Could not parse config file");
            std::process::exit(1);
        }
    };

    match matches.values_of("files") {
        Some(_) => use_files(matches, &conf).await,
        None => use_stdin(matches, &conf).await,
    }
}

async fn use_files(matches: ArgMatches, conf: &config::Conf) {
    let color_choice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stdout = StandardStream::stdout(color_choice);
    let mut stderr = StandardStream::stderr(color_choice);

    let check = matches.is_present("check");
    let write = matches.is_present("write");
    let fail_fast = matches.is_present("fail_fast");
    let files = matches.values_of("files").unwrap();
    let parser = matches.value_of("parser");

    let mut futures: FuturesUnordered<_> = FuturesUnordered::new();
    for filename in utils::get_files(files) {
        futures.push(process::run_file(&conf, filename, parser, check, write));
    }

    let mut error_count = 0;
    let mut ok_count = 0;
    let mut changed_count = 0;

    while let Some(result) = futures.next().await {
        match result {
            process::FormatResult::Unchanged(f) => {
                ok_count += 1;
                if !check && !write {
                    utils::print_ok(&mut stdout);
                    print!("{f}\n");
                }
            }
            process::FormatResult::Changed(f) => {
                changed_count += 1;
                if check {
                    eprintln!("{f}")
                } else if write {
                    utils::print_ok(&mut stdout);
                    print!("{f}\n");
                } else {
                    utils::print_fail(&mut stderr);
                    eprint!("{f}\n");
                }
                if !write && fail_fast {
                    println!("Failed fast...");
                    break;
                }
            }
            process::FormatResult::Err(e) => {
                error_count += 1;
                if check {
                    let filename = match &e.filename {
                        Some(f) => f,
                        None => "Unknown",
                    };
                    eprintln!("{filename}");
                } else {
                    utils::print_error(&mut stderr);
                    eprint!("{}", e.to_string());
                }
                if fail_fast {
                    println!("Failed fast...");
                    break;
                }
            }
            process::FormatResult::Ignored => (),
        }
    }

    if write {
        println!(
            "\nWritten {changed_count} {}.",
            if changed_count == 1 { "file" } else { "files" }
        );
    }

    if !write && !check {
        println!(
            "\n{ok_count}/{} files are formatted correctly.",
            ok_count + changed_count + error_count
        );
    }

    if error_count > 0 || (changed_count > 0 && check) {
        std::process::exit(1);
    }
}

async fn use_stdin(matches: ArgMatches, conf: &config::Conf) {
    let check = matches.is_present("check");
    let parser = matches
        .value_of("parser")
        .expect("--parser is required for Stdin.");

    match process::run_stdin(conf, parser, check).await {
        process::FormatResult::Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
        _ => (),
    }
}
