# JSONA - JSON with annotations

The JSONA Data Interchange Format (JSONA) is a superset of [JSON] that supports annotations. It also aims to
alleviate some of the limitations of JSON by expanding its syntax and 

## Summary of Features

### Annotations

### Objects
- Object keys may be an ECMAScript IdentifierName.
- Objects may have a single trailing comma.

### Arrays
- Arrays may have a single trailing comma.

### Strings
- Strings may be single quoted.
- Strings may span multiple lines by escaping new line characters.
- Strings may include character escapes.

### Numbers
- Numbers may be hexadecimal.
- Numbers may have a leading or trailing decimal point.

### Comments
- Single and multi-line comments are allowed.

### White Space
- Additional white space characters are allowed.

## Short Example
```
@doc // doc level annotation
@swagger(version="2") // annotation with args

{
  // comments
  unquoted: 'and you can quote me on that',
  singleQuotes: 'I can use "double quotes" here',
  lineBreaks: "Look, Mom! \
No \\n's!",
  hexadecimal: 0xdecaf, @optional
  leadingDecimalPoint: .8675309, andTrailing: 8675309.,
  positiveSign: 1,
  trailingComma: 'in objects', andIn: ['arrays',],
  "backwardsCompatible": "with JSON",
}
```
