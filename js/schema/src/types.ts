import { type JSONSchema } from "json-schema-typed/draft-2019-09";

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

export interface ParseResult {
  schema?: JSONSchema,
  errors?: ErrorObject[],
}

export { JSONSchema };