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
