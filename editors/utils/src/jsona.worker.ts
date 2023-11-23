import {
  BrowserMessageReader,
  BrowserMessageWriter,
} from "vscode-languageserver-protocol/browser";

import { createLsp, JsonaWasmLsp, RpcMessage, utilTypes } from "@jsona/lsp";
import { createLogger, createRpc } from "@jsona/lsp";

const worker: Worker = self as any;

const writer = new BrowserMessageWriter(worker);
const reader = new BrowserMessageReader(worker);

let lsp: JsonaWasmLsp;
let rootUri = "file:///";
const logger = createLogger({
  debug: false,
  topics: "*",
});

const log = logger.log;
const rpc = createRpc({
  write: v => writer.write(v),
  log
});

reader.listen(async (message: RpcMessage) => {
  if (!lsp) {
    lsp = createLsp(
      utilTypes.convertEnv(
      {
        envVar: (name) => {
          if (name === "RUST_LOG") {
            return logger.level();
          } else {
            return "";
          }
        },
        now: () => new Date(),
        readFile: rpc.readFile,
        writeFile: () => Promise.reject("not implemented write_file"),
        stderr: async (bytes: Uint8Array) => {
          console.log(new TextDecoder().decode(bytes));
          return bytes.length;
        },
        stdErrAtty: () => false,
        stdin: () => Promise.reject("not implemented stdin"),
        stdout: async (bytes: Uint8Array) => {
          console.log(new TextDecoder().decode(bytes));
          return bytes.length;
        },
        fetchFile: async (url) => {
          log("fetchFile", url);
          const controller = new AbortController();
          const timeout = setTimeout(() => {
            controller.abort();
          }, 30000);
          try {
            const res = await fetch(url, { signal: controller.signal });
            const buf = await res.arrayBuffer();
            return new Uint8Array(buf)
          } catch (err) {
            throw err;
          } finally {
            clearTimeout(timeout);
          }
        },
        rootUri: () => rootUri,
      }),
      {
        js_on_message: (message) => {
          log('lsp2host', message);
          writer.write(message);
        },
      }
    );
  }

  log('host2lsp', message);
  if (!rpc.recv(message)) {
    if (message.method === "lsp/debug") {
      logger.debug = true;
      return;
    }
    lsp.send(message);
  }
});
