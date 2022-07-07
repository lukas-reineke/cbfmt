# Codeblock Format

Format codeblocks inside markdown and org documents.  
It iterates over all codeblocks, and formats them with the tool specified for
that language.

## Install

TBD

## Config

A configuration file is required. By default the file is called
`.codeblock-format.toml`

#### languages

This section specifies which commands should run for which language.  
Each entry is the name of the language as the key, and a list of format commands
to run in sequence as the value.

Example:

```toml
[languages]
lua = ["stylua -s -"]
go = ["goimports"]
```

## Usage

### With arguments

You can run `codeblock-format` on files and or directories by passing them as
arguments.

```bash
codeblock-format [OPTIONS] [file/dir/glob]...
```

The default behaviour checks formatting for all files that were passed as
arguments. If all files are formatted correctly, it exits with status code 0,
otherwise it exits with status code 1.

### With Stdin

If no arguments are specified, `codeblock-format` will read from stdin and write
to stdout.

```bash
echo "$markdown" | codeblock-format [OPTIONS]
```

### Options

These are the most important options. To see all options, please run
`codeblock-format --help`

#### check `-c|--check`

Works the same as the default behaviour, but only prints the path to files that
fail.

#### write `-w|--write`

Writes the format result back into the files.

#### parser `-p|--parser`

Specifies which parser to use. This is required for stdin, but inferred from the
file ending when files are passed as arguments.
