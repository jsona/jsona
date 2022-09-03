import loadCrate from "../../../crates/jsona-wasm-cli/Cargo.toml";
import { convertEnv, Environment, prepareEnv } from "@jsona/util-types";
export * as types from "./types";

export interface RpcMessage {
  jsonrpc: "2.0";
  method?: string;
  id?: string | number;
  params?: any;
  result?: any;
  error?: any;
}

export interface LspInterface {
  /**
   * Handler for RPC messages set from the LSP server.
   */
  onMessage: (message: RpcMessage) => void;
}
export default class JsonaLsp {
  private static jsona: any | undefined;
  private static guard: boolean = false;

  private constructor(private env: Environment, private lspInner: any) {
    if (!JsonaLsp.guard) {
      throw new Error(
        `an instance of Jsona can only be created by calling the "getInstance" static method`
      );
    }
  }

  public static async getInstance(
    env: Environment,
    lspInterface: LspInterface
  ): Promise<JsonaLsp> {
    if (typeof JsonaLsp.jsona === "undefined") {
      JsonaLsp.jsona = await loadCrate()
    }
    JsonaLsp.jsona.initialize();

    prepareEnv(env);

    JsonaLsp.guard = true;
    const t = new JsonaLsp(
      env,
      JsonaLsp.jsona.create_lsp(convertEnv(env), {
        js_on_message: lspInterface.onMessage,
      })
    );
    JsonaLsp.guard = false;

    return t;
  }

  public send(message: RpcMessage) {
    this.lspInner.send(message);
  }

  public dispose() {
    this.lspInner.free();
  }
}
