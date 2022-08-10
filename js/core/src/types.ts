/**
 * Jsona formatter options.
 */
export interface FormatOptions {
  /// Indentation to use, should be tabs or spaces
  /// but technically could be anything.
  indent_string?: string,

  /// Put trailing commas for multiline arrays/objects
  trailing_comma?: boolean,

  /// Add trailing newline to the source.
  trailing_newline?: boolean,
}


export interface ToAstResult {
  ast?: Ast,
  errors?: ToAstError[],
}

export type Ast = AstObject | AstArray | AstString | AstNumber | AstBool | AstNull;

export interface AstObject {
  type: "object",
  properties: AstProperty[],
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstArray {
  type: "array",
  items: Ast[],
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstString {
  type: "string",
  value: string,
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstNumber {
  type: "number",
  value: number,
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstBool {
  type: "bool",
  value: boolean,
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstNull {
  type: "null",
  annotations: AstAnnotation[],
  range?: AstRange;
}

export interface AstProperty {
  type: AstKey,
  value: Ast,
}

export interface AstAnnotation {
  type: AstKey,
  value: Ast,
}

export interface AstAnnotationValue {
  value: any,
  range?: AstRange,
}

export interface AstKey {
  name: string,
  range?: AstRange,
}

export interface AstRange {
  start: AstPosition;
  end: AstPosition;
}

export interface AstPosition {
  index: number;
  line: number;
  character: number;
}

export interface ToAstError {
  kind: string,
  message: string,
  range?: AstRange,
}
