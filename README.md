# JSONA - JSON with annotations

The JSONA Data Interchange Format (JSONA) is a superset of [JSON] that supports annotations. It also aims to
alleviate some of the limitations of JSON by expanding its syntax and 

## Summary of Features

### Annotations
- Annotation add metadata to the data
- Annotation synatx: `@name(jsonvalue)`

### Objects
- Object keys may be an ECMAScript IdentifierName.
- Objects may have a single trailing comma.

### Arrays
- Arrays may have a single trailing comma.

### Strings
- Strings may be single quoted.
- Strings may span multiple lines by escaping new line characters.
- Strings may include character escapes.
- String may be backtick quoted.

### Numbers
- Numbers may be hexadecimal.
- Numbers may have a leading or trailing decimal point.

### Comments
- Single and multi-line comments are allowed.

### White Space
- Additional white space characters are allowed.

## Example
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
    @object({k: "v"})

    nullValue: null,
    boolTrue: true,
    boolFale: false,
    float: 3.14,
    floatNegative: -3.14,
    floatNegativeWithoutInteger: -.14,
    floatNegativeWithoutDecimal: -3.,
    integer: 3,
    hex: 0x1a,
    binary: 0b01,
    otcal: 0o12,
    integerNegative: -3,
    stringSingleQuota: 'abc "def" ghi',
    stringDoubleQuota: "abc 'def' ghi",
    stringBacktick: `abc
def \`
xyz`,
    stringEscaple1: '\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}',
    stringEscaple2: "\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}",
    stringEscaple3: `\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}`,
    arrayEmpty: [], 
    arrayEmptyMultiLine: [ @array
    ],
    arrayEmptyWithAnnotation: [],  // @array
    arraySimple: [ @array
        "a", @upper
        "b",
    ],
    arrayOneline: ["a", "b"], @array
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
