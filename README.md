# cbfmt (codeblock format)

A tool to format codeblocks inside markdown, org, and restructuredtext documents.  
It iterates over all codeblocks, and formats them with the tool(s) specified for
the language of the block.

## Install

### Download from GitHub

Download the latest release binaries from [github.com/lukas-reineke/cbfmt/releases](https://github.com/lukas-reineke/cbfmt/releases)

### Cargo

```bash
cargo install cbfmt
```

### Build from source

1. Clone this repository
2. Build with [cargo](https://github.com/rust-lang/cargo/)

```bash
git clone https://github.com/lukas-reineke/cbfmt.git && cd cbfmt
cargo install --path .
```

This will install `cbfmt` in your `~/.cargo/bin`. Make sure to add `~/.cargo/bin` directory to your `PATH` variable.

## Config

A configuration file is required. By default the file is called
`.cbfmt.toml`

Example:

```toml
[languages]
rust = ["rustfmt"]
go = ["gofmt"]
lua = ["stylua -s -"]
python = ["black --fast -"]
```

### Sections

#### languages

This section specifies which commands should run for which language.  
Each entry is the name of the language as the key, and a list of format commands
to run in sequence as the value. Each format command needs to read from stdin
and write to stdout.

## Usage

### With arguments

You can run `cbfmt` on files and or directories by passing them as
arguments.

```bash
cbfmt [OPTIONS] [file/dir/glob]...
```

The default behaviour checks formatting for all files that were passed as
arguments. If all files are formatted correctly, it exits with status code 0,
otherwise it exits with status code 1.

When a directory is passed as an argument, `cbfmt` will recursively run on all files
in that directory which have a valid parser and are not ignored by git.

### With stdin

If no arguments are specified, `cbfmt` will read from stdin and write the format
result to stdout.

```bash
cbfmt [OPTIONS] < [file]
```

### Without arguments and stdin

If there are no arguments and nothing is written to stdin, `cbfmt` will print
the help text and exit.

### Options

These are the most important options. To see all options, please run
`cbfmt --help`

#### check `-c|--check`

Works the same as the default behaviour, but only prints the path to files that
fail.

#### write `-w|--write`

Writes the format result back into the files.

#### parser `-p|--parser`

Specifies which parser to use. This is inferred from the file ending when
possible.
