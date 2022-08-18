# Command Line

JSONA CLI aims to be an one stop shop tool for working with JSONA files via the command line. The features include validation, formatting, and querying JSONA documents with a jq-like fashion.

## Installation

### Binary Releases

We pre-compile each release for all major operating systems, these releases can be found on [GitHub Releases](https://github.com/jsona/jsona/releases).

### Cargo

If you have a Rust toolchain installed, you can install CLI via the jsona-cli crate from crates.io.

```
cargo install jsona-cli --locked
```

> Make sure to use `--locked` if you run into weird compile errors due to incompatible dependencies.

## Usage

```
USAGE:
    jsona [OPTIONS] <SUBCOMMAND>

OPTIONS:
        --colors <COLORS>    [default: auto] [possible values: auto, always, never]
    -h, --help               Print help information
        --log-spans          Enable logging spans
    -V, --version            Print version information
        --verbose            Enable a verbose logging format

SUBCOMMANDS:
    format    Format JSONA documents [aliases: fmt]
    get       Extract a value from the given JSONA document
    help      Print this message or the help of the given subcommand(s)
    lint      Lint JSONA documents [aliases: check, validate]
    lsp       Language server operations
```
### Configuration

#### Log Level

JSONA CLI uses the Rust `tracing` library for configurable logging features and respects the `RUST_LOG` environment variable. All logs regardless of log level are printed to the standard error output.

In most cases you might wish to disable logging below a certain log level.
As an example if you wish to only see error messages, you can do the following:

```sh
RUST_LOG=error jsona lint foo.jsona
```

The available log levels:

- `trace`
- `debug`
- `info`
- `warn`
- `error`


### Validation

JSONA CLI supports validation of JSONA files, by default it will only look for syntax errors and some semantic errors such as duplicate keys.

```sh
jsona lint foo.jsona
```

#### Schema Validation

JSONA supports validation via [JSONA Schemas](crates/jsona-schema-validator).

```sh
jsona lint --schema https://example.com/foo-schema.json foo.jsona
```

### Formatting

It is possible to format files in-place or via standard i/o.

```sh
jsona fmt foo.jsona
```

Or

```sh
cat foo.jsona | jsona fmt -
```

> By default JSONA CLI will bail on documents that contain syntax errors to avoid destructive edits, you can use the `--force` flag to suppress this and try to format the invalid document(s) anyway.

#### Options

Please check [formatter options](./config.md#formatter-options) for more details, it is possible to specify overrides via the `--option` flag:

```sh
jsona fmt --option trailing_comma=true foo.jsona
```

#### Check

It is possible to check whether the given files are properly formatted via the `--check` flag. When this flag is supplied, no formatting will be done.


### Querying

It is possible to query specific values via a simple query expressions.

```
jsona get -f foo.jsona 'foo[1].bar'
```

This will print value in plain json. Use option `-A` to print json with annotations.

### Language Server

The JSONA language server can be used via the CLI and it supports communication via standard i/o or TCP.

#### Via Standard i/o

```
jsona lsp stdio
```

In this mode JSONA CLI expects messages from the standard input, and will print messages intended for the client to the standard output.

#### Via TCP

```
jsona lsp tcp --address 0.0.0.0:9181
```

The server will listen on the given TCP address.

Multiple clients are not supported.