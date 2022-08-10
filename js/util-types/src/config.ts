import { FormatOptions } from "./formatter";

export interface Config {
  /**
   * Files to include.
   *
   * A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns. Globstars (`**`) are supported.
   *
   * Relative paths are **not** relative to the configuration file, but rather depends on the tool using the configuration.
   *
   * Omitting this property includes all files, **however an empty array will include none**.
   */
  include?: string[];
  /**
   * Files to exclude (ignore).
   *
   * A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns. Globstars (`**`) are supported.
   *
   * Relative paths are **not** relative to the configuration file, but rather depends on the tool using the configuration.
   *
   * This has priority over `include`.
   */
  exclude?: string[];
  /**
   * Formatting options.
   */
  formatting?: FormatOptions;
  /**
   * Rules are used to override configurations by path and keys.
   */
  rules?: Rule[];
}

/**
 * A rule to override options by either name or file.
 */
export interface Rule {
  /**
   * The name of the rule.
   */
  name?: string;
  /**
   * Files this rule is valid for.
   *
   * A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
   *
   * Relative paths are **not** relative to the configuration file, but rather depends on the tool using the configuration.
   *
   * Omitting this property includes all files, **however an empty array will include none**.
   */
  include?: string[];
  /**
   * Files that are excluded from this rule.
   *
   * A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
   *
   * Relative paths are **not** relative to the configuration file, but rather depends on the tool using the configuration.
   *
   * This has priority over `include`.
   */
  exclude?: string[];
  /**
   * A local file path to the schema, overrides `url` if set.
   *
   * For URLs, please use `url` instead.
   */
  path?: string;
  /**
   * A full absolute Url to the schema.
   *
   * The url of the schema, supported schemes are `http`, `https`, `file` and `jsona`.
   */
  url?: string;
  /**
   * Formatting options.
   */
  formatting?: FormatOptions;
}