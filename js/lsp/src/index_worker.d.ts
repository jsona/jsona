import * as utilTypes from "@jsona/util-types";

/**
 * Format the given JSONA document.
 *
 * @param jsona JSONA document.
 * @param options Optional format options.
 */
export function format(input: string, options: utilTypes.FormatOptions): string;

/**
 * Lint the given JSONA document
 * @param env Environment
 * @param jsona JSONA document
 * @param schema_url JsonaSchema Uri
 */
export function lint(
  env: utilTypes.Environment,
  input: string,
  schema_url: string
): Promise<utilTypes.ErrorObject>;

/**
 * Create lsp server
 * @param env Environment
 * @param lsp_interface Lsp interface object
 */
export function createLsp(env: utilTypes.Environment, lsp_interface: LspInterface): JsonaWasmLsp;

export class JsonaWasmLsp {
  free(): void;
  send(message: RpcMessage): void;
}

export interface LspInterface {
  /**
   * Handler for RPC messages set from the LSP server.
   */
  js_on_message: (message: RpcMessage) => void;
}

export interface RpcMessage {
  jsonrpc: "2.0";
  method?: string;
  id?: string | number;
  params?: any;
  result?: any;
  error?: any;
}

export interface ServerNotifications {
  "jsona/messageWithOutput": {
    params: {
      kind: "info" | "warn" | "error";
      message: string;
    };
  };
  "jsona/initializeWorkspace": {
    params: {
      rootUri: string
    };
  };
}

export type ServerNotificationsParams<T extends keyof ServerNotifications> =
  ServerNotifications[T] extends WithParams
  ? ServerNotifications[T]["params"]
  : never;

export interface ClientNotifications {

}

export type ClientNotificationsParams<T extends keyof ClientNotifications> =
  ClientNotifications[T] extends WithParams
  ? ClientNotifications[T]["params"]
  : never;

export interface ClientRpc {
  "jsona/listSchemas": {
    params: {
      documentUri: string;
    };
    response: {
      schemas: Array<SchemaInfo>;
    };
  };
  "jsona/associatedSchema": {
    params: {
      documentUri: string;
    };
    response: {
      schema?: SchemaInfo | null;
    };
  };
}


export type ClientRpcParams<T extends keyof ClientRpc> =
  ClientRpc[T] extends WithParams ? ClientRpc[T]["params"] : never;

export type ClientRpcResponses<T extends keyof ClientRpc> =
  ClientRpc[T] extends WithResponse ? ClientRpc[T]["response"] : never;

export interface SchemaInfo {
  url: string;
  meta: any;
}

interface WithParams {
  readonly params: any;
}

interface WithResponse {
  readonly response: any;
}

