/**
 * Parse jsona doc as json
 * @param input JSONA document.
 */
export function parse(input: string): CoreTypes.ParseResult;

/**
 * Parse jsona doc as ast
 * @param input JSONA document.
 */
export function parseAst(input: string): CoreTypes.ParseAstResult;

/**
 *  Stringify ast to jsona doc
 * @param ast JSONA ast.
 */
export function stringifyAst(ast: CoreTypes.Ast): String;

/**
 * Format the given JSONA document.
 *
 * @param jsona JSONA document.
 * @param options Optional format options.
 */
export function format(input: string, options: CoreTypes.FormatOptions): string;

export namespace CoreTypes {
  export interface Range {
    start: Position;
    end: Position;
  }

  export interface Position {
    index: number;
    line: number;
    character: number;
  }

  export interface ErrorObject {
    kind: string,
    message: string,
    range?: Range,
  }

  /**
   * Jsona formatter options.
   */
  export interface FormatOptions {
    /// Indentation to use, should be tabs or spaces
    /// but technically could be anything.
    indent_string?: string,

    /// Put trailing commas for multiline arrays/objects.
    trailing_comma?: boolean,

    /// Add trailing newline to the source.
    trailing_newline?: boolean,

    /// Remove unnecessary quote or choose better quote for property.
    format_key?: boolean,
  }


  export interface ParseResult {
    value?: any,
    errors?: ErrorObject[],
  }

  export interface ParseAstResult {
    value?: Ast,
    errors?: ErrorObject[],
  }

  export type Ast = AstObject | AstArray | AstString | AstNumber | AstBool | AstNull;

  export interface AstObject {
    type: "object",
    properties: AstProperty[],
    annotations: AstAnnotation[],
    range?: Range;
  }

  export interface AstArray {
    type: "array",
    items: Ast[],
    annotations: AstAnnotation[],
    range?: Range;
  }

  export interface AstString {
    type: "string",
    value: string,
    annotations: AstAnnotation[],
    range?: Range;
  }

  export interface AstNumber {
    type: "number",
    value: number,
    annotations: AstAnnotation[],
    range?: Range;
  }

  export interface AstBool {
    type: "bool",
    value: boolean,
    annotations: AstAnnotation[],
    range?: Range;
  }

  export interface AstNull {
    type: "null",
    annotations: AstAnnotation[],
    range?: Range;
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
    range?: Range,
  }

  export interface AstKey {
    name: string,
    range?: Range,
  }
}
