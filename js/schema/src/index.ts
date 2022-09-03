import loadCrate from "../../../crates/jsona-wasm-schema/Cargo.toml";
import { ParseResult } from "./types";
export * as types from "./types";

export default class JsonaSchema {
  private static crate: any | undefined;
  private static guard: boolean = false;
  private constructor() {
    if (!JsonaSchema.guard) {
      throw new Error(
        `an instance of JsonaSchema can only be created by calling the "getInstance" static method`
      );
    }
  }

  public static async getInstance(): Promise<JsonaSchema> {
    if (typeof JsonaSchema.crate === "undefined") {
      JsonaSchema.crate = await loadCrate();
    }
    JsonaSchema.guard = true;
    const self = new JsonaSchema();
    JsonaSchema.guard = false;
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
