# Jsona Schema

A jsonaschmea is mainly used for: 

- verify jsona document
- code completions

## Developing Schemas

See [schemastore](https://github.com/jsona/schemastore/README.md) for more details.

## Using schema

JSONA schemas can be assigned to JSONA documents according to the following in priority order starting with the highest priority:

- set manually in the environment, e.g. as [a CLI flag](./cli.md#using-a-specific-schema) or an IDE setting
- as an URL under the `@jsonaschema` annotation in the root of the document
- configuration file [rule](./config.md#rules)
- contributed by an [extension](#visual-studio-code-extensions) *(Visual Studio Code only)*

## Visual Studio Code extensions

Similarly to [`jsonValidation`](https://code.visualstudio.com/api/references/contribution-points#contributes.jsonValidation), it is possible for extensions to contribute their own schemas.

Other than `fileMatch`, it is also possible to specify `regexMatch` that is matched against the entire document URI.

```json
{
  "contributes": {
    "jsonaValidation": [
      {
        "regexMatch": "^.*foo.jsona$",
        "url": "https://json.schemastore.org/foo.json"
      }
    ]
  }
}
```
