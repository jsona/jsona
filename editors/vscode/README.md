
A JSONA language support extension with lsp supported.

It is currently a **preview extension**, it might contain bugs, or might even crash. If you encounter any issues, please report them [on github](https://github.com/jsona/jsona/issues).

- [Features](#features)
  - [Syntax highlighting](#syntax-highlighting)
  - [Validation](#validation)
  - [Folding](#folding)
  - [Symbol tree and navigation](#symbol-tree-and-navigation)
  - [Formatting](#formatting)
  - [Completion and Validation With JSONA Schema](#completion-and-validation-with-jsona-schema)
- [Configuration File](#configuration-file)

# Features

## Syntax highlighting

Syntax highlighting for JSONA documents with TextMate grammar.

![Syntax Highlighting](images/highlight.png)

## Validation

![Validation](images/validation.gif)

## Folding

Arrays, multi-line strings and top level tables and comments can be folded.

![Folding](images/folding.gif)

## Symbol tree and navigation

Works even for tables not in order.

![Symbols](images/symbols.gif)

## Formatting

The formatter is rather conservative by default, additional features can be enabled in the settings. If you're missing a configuration option, feel free to open an issue about it!

![Formatting](images/formatting.gif)

## Completion and Validation With JSONA Schema

There is support for completion, hover text, links and validation.

Schemas can be associated with document URIs with the `jsona.schema.associations` configuration.

![Schema](images/schema.gif)


# Configuration File

Jsona CLI's [configuration file](https://github.com/jsona/jsona/blob/main/docs/config-file.md) is supported and automatically found in workspace roots, or can be manually set in the VS Code configuration.
