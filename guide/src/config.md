# Configuration

## Configuration File

JSONA supports configuration via file, unsurprisingly it uses the JSONA format.

By default, every tool looks for one in the working directory or the root of the workspace by `.jsona`

### Include

The `include` property is an array of [glob](<https://en.wikipedia.org/wiki/Glob_(programming)>) path strings that are relative to the working directory (or root of the workspace),
the matched files are included in the operations by the tools unless explicitly overwritten. The pattern supports globstars (`**`) for recursive search.

If this property is omitted, `JSONA` files will be searched in the entire child directory tree from the root.


> If `include` is present but empty, **no files will be included**.

```jsona
include = ["api.jsona", "some_directory/**/*.jsona"]
```

### Exclude

The `exclude` property has the same semantics as `include` and takes precedence over it.

The following will exclude `mixin.jsona` from the includes written above, so files matching `some_directory/**/*.jsona` will be included only.

```jsona
exclude = ["mixin.jsona"]
```

### Formatting Options

The `formatting` table contains optional [formatting options](#formatting-options) for the formatter:

```jsona
{
  tailing_comma: false,
}
```

### Rules

The `rule` array of tables consist of rules that overwrite the above configuration based on some conditions.
Thus it has the same `formatting` and `schema` settings, and the `include` and `exclude` with the same semantics as their [global variants](#include), however this time they are used to determine whether the rule applies.

> In case of overlapping rules, the last defined rule always takes precedence.

Let's say we want to sort our `Cargo` dependencies, but nothing else, here is how we would do that:

```jsona
{
  formatting: {
    tailing_comma: false,
  },
  rules: [
    {
      name: "openapi",
      include: ["api*.jsona"],
      path: "schemastore/openapi.schema.jsona",
    },
  ],
}
```

## Formatter Options

This page contains a list of formatting options the formatter accepts.


> In some environments (e.g. in Visual Studio Code and JavaScript) the option keys are *camelCase* to better fit the conventions. For example `tailing_comma` becomes `tailingComma`.

| option           | description                                                                    | default  |
| :--------------- | :----------------------------------------------------------------------------- | :------- |
| indent_string    | Indentation to use, should be tabs or spaces but technically could be anything | 2 spaces |
| trailing_comma   | Put trailing commas for multiline arrays/objects                               | true     |
| trailing_newline | Add trailing newline to the source                                             | true     |
| format_key       | Remove unnecessary quote or choose better quote for property.                  | false    |