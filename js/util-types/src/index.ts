export * from "./environment";
export * from "./formatter";

/**
 * Byte range within a JSONA document.
 */
export interface Range {
  /**
   * Start byte index.
   */
  start: number;
  /**
   * Exclusive end index.
   */
  end: number;
}


export interface ErrorObject {
  kind: string,
  message: string,
  range?: Range,
}