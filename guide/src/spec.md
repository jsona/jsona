# Specification

- [Specification](#specification)
  - [Introduction](#introduction)
  - [Example](#example)
  - [JSON](#json)
    - [Support for comments](#support-for-comments)
    - [Use quotes freely on property key](#use-quotes-freely-on-property-key)
    - [Allow extra trailing commas](#allow-extra-trailing-commas)
    - [Omit part of floating point numbers](#omit-part-of-floating-point-numbers)
    - [Multiple bases support](#multiple-bases-support)
    - [Support single quotes and backtick quotes](#support-single-quotes-and-backtick-quotes)
    - [Multi-line string](#multi-line-string)
    - [Escape string](#escape-string)
  - [Annotation](#annotation)
    - [Insert position](#insert-position)
    - [Annotation value](#annotation-value)


## Introduction

JSONA = JSON + Annotation. JSON describes the data, Annotation describes the logic.

## Example

The examples below cover all the features of JSONA.

```
/*
 multiple line comment
*/

// single line comment

{
    @foo /* abc */ @optional
    @null(null) // single line comment
    @bool(true)
    @float(3.14)
    @number(-3)
    @string('abc "def" ghi')
    @array([3,4])
    @object({
        k1: "v1",
        k2: "v2",
    })

    nullValue: /* xyz */ null,
    boolTrue: true,
    boolFalse: false,
    float: 3.14,
    floatNegative: -3.14,
    floatNegativeWithoutInteger: -.14,
    floatNegativeWithoutDecimal: -3.,
    integer: 3,
    hex: 0x1a,
    binary: 0b01,
    octal: 0o12,
    integerNegative: -3,
    stringSingleQuota: 'abc "def" ghi',
    stringDoubleQuota: "abc 'def' ghi",
    stringBacktick: `abc
def \`
xyz`,
    stringEscape1: '\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}',
    stringEscape2: "\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}",
    stringEscape3: `\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}`,
    arrayEmpty: [], 
    arrayEmptyMultiLine: [ @array
    ],
    arrayEmptyWithAnnotation: [],  // @array
    arraySimple: [ @array
        "a", @upper
        "b",
    ],
    arrayOnline: ["a", "b"], @array
    arrayExtraComma: ["a", "b",],
    objectEmpty: {},
    objectEmptyMultiLine: { @object
    },
    objectEmptyWithAnnotation: {}, @use("Object4")
    objectSimple: { @save("Object4")
        k1: "v1", @upper
        k2: "v2",
    },
    objectOneLine: { k1: "v1", k2: "v2" }, @object
    objectExtraComma: { k1: "v1", k2: "v2", },
}
```

## JSON

JSONA is a superset of JSON, borrowing the syntax of ECMAScript to alleviate some of the limitations of JSON.

### Support for comments

```
/**
 multiple lines comment
*/
// single line  comment
{
  @anno /* inline comment */ @anno
}
```

### Use quotes freely on property key

```
{
  "a": 1,
  b: 2,
  'a': 3,
  `a`: 4,
}
```

### Allow extra trailing commas

```
{
  a: 3,
  b: 4,
  c: [
    'x',
    'y',
  ],
}
```

### Omit part of floating point numbers

```
{
  a: 3.,
  b: .1,
  c: 3.1,
}
```

### Multiple bases support

```
{
  integer: 3,
  hex: 0x1a,
  binary: 0b01,
  octal: 0o12,
}
```

### Support single quotes and backtick quotes

```
{
  x: 'abc "def" ghi',
  y: "abc 'def' ghi",
  z: `abc "def", 'ghi'`,
}
```


### Multi-line string

```
{
  x: `abc
  def`
}
```

### Escape string

```
{
  x: '\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}', // single quote
  y: "\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}", // double quote
  z: `\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}`, // backtick quote
}
```


## Annotation

Annotations are marked with `@` followed by a variable name. Annotations may or may not have value.

### Insert position

Here's a list of where all the annotations are in JSONA:

```
{ @anno 
  @anno
  v1: 1, @anno
  v2: {}, @anno
  v3: [], @anno
  v4: [ @anno
  ],
  v5: [
    @anno
  ],
  v6: [
  ], @anno
} @anno
@anno
```

### Annotation value

Annotation values ​​must be enclosed in parentheses, but can be omitted.

Annotation values ​​must be valid but no annotation JSONA, annotation values ​​cannot nest annotation values.

```
@anno
@anno(null)
@anno(true)
@anno('a')
@anno(3)
@anno(0.3)
@anno([])
@anno(['a'])
@anno({})
@anno({a:3})
```