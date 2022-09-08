import { JSONSchema } from "json-schema-typed/draft-2019-09";

/**
 * Parse jsona doc as schema
 * @param input JSONA document.
 */
export function parse(input: string): SchemaTypes.ParseResult;

export namespace SchemaTypes {
  export import Schema = JSONSchema;

  export interface Range {
    start: Position;
    end: Position;
  }

  export interface Position {
    index: number;
    line: number;
    column: number;
  }

  export interface ErrorObject {
    kind: string,
    message: string,
    range?: Range,
  }

  export interface ParseResult {
    value?: JSONSchema,
    errors?: ErrorObject[],
  }
}