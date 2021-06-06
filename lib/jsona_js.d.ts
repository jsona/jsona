/* tslint:disable */
/* eslint-disable */
/**
* @param {string} data
* @returns {any}
*/
export function parse(data: string): ParseResult<JsonaValue>;
/**
* @param {string} data
* @returns {any}
*/
export function parseAsJSON(data: string): ParseResult<any>;

export interface ParseResult<T> {
  jsona: T,
  error: JsonaError,
}

export interface JsonaError {
  info: string;
  position: Position;
}

export type JsonaValue = 
  JsonaNull |
  JsonaBoolean |
  JsonaFloat |
  JsonaInteger |
  JsonaString |
  JsonaObject |
  JsonaArray;

export interface Position {
  index: number;
  line: number;
  col: number;
}

export interface JsonaAnnotation {
  name: string;
  position: Position,
  value: any;
}

export interface JsonaProperty {
  key: string;
  position: Position;
  value: JsonaValue;
}

export interface JsonaValueBase {
  annotations: JsonaAnnotation[];
  position: Position;
}

export interface JsonaNull extends JsonaValueBase {
  type: "Null";
}

export interface JsonaBoolean extends JsonaValueBase {
  type: "Boolean";
  value: boolean;
}

export interface JsonaFloat extends JsonaValueBase {
  type: "Float";
  value: number;
}

export interface JsonaInteger extends JsonaValueBase {
  type: "Integer";
  value: number;
}

export interface JsonaString extends JsonaValueBase {
  type: "String";
  value: string;
}

export interface JsonaObject extends JsonaValueBase {
  type: "Object";
  properties: JsonaProperty[];
}

export interface JsonaArray extends JsonaValueBase {
  type: "Array";
  elements: JsonaValue[];
}

