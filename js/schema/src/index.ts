import loadCrate from "../../../crates/jsona-wasm-schema/Cargo.toml";
import { ParseResult } from "./types";
export * as types from "./types";

export class JsonaSchema {
  private static crate: any | undefined;
  private static initializing: boolean = false;
  private constructor() {
    if (!JsonaSchema.initializing) {
      throw new Error(
        `an instance of JsonaSchema can only be created by calling the "initialize" static method`
      );
    }
  }

  public static async init(): Promise<JsonaSchema> {
    if (typeof JsonaSchema.crate === "undefined") {
      JsonaSchema.crate = await loadCrate();
    }
    JsonaSchema.initializing = true;
    const self = new JsonaSchema();
    JsonaSchema.initializing = false;
    return self;
  }

  /**
   * Parse jsona doc as ast
   * @param jsona JSONA document.
   */
  public parse(jsona: string): ParseResult {
    try {
      return { schema: JsonaSchema.crate.parse(jsona) }
    } catch (errors) {
      return { errors: errors }
    }
  }
}
