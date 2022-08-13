# Jsona Schema

A jsonaschmea is mainly used for: 

- verify jsona document
- code completions

## Developing Schemas

Schemastore is a universal JSON schema store where schemas for popular JSONA schema documents can be found.

See [schemastore](https://github.com/jsona/schemastore/README.md) for more details.

## Using schema

JSONA schemas can be assigned to JSONA documents according to the following in priority order starting with the highest priority:

- set manually in the environment, e.g. as [a CLI flag](./cli.md#using-a-specific-schema) or an IDE setting
- as an URL under the `@jsonaschema` in the root of the document
- [configuration file rules](./config.md#rules)
- contributed by an [extension](#visual-studio-code-extensions) *(Visual Studio Code only)*
- an association based on a schemastore

## Visual Studio Code extensions

Similarly to [`jsonValidation`](https://code.visualstudio.com/api/references/contribution-points#contributes.jsonValidation), it is possible for extensions to contribute their own schemas.

Other than `fileMatch`, it is also possible to specify `regexMatch` that is matched against the entire document URI.

```json
{
  "contributes": {
    "jsonaValidation": [
      {
        "regexMatch": "^.*api.jsona$",
        "url": "https://cdn.jsdelivr.net/npm/@jsona/schemastore@0.1.2/openapi.jsona"
      }
    ]
  }
}
```
