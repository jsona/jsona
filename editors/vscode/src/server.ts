import { exit } from "process";
import fs from "fs/promises";
import { pathToFileURL, fileURLToPath } from "url";
import { createLsp, RpcMessage, utilTypes, JsonaWasmLsp } from "@jsona/lsp";
import fetch, { Headers, Request, Response } from "node-fetch";
import { createRpc, createLogger } from "@jsona/lsp";

let lsp: JsonaWasmLsp;

const logger = createLogger({
  debug: import.meta.env.RUST_LOG === "debug",
  topics: import.meta.env.LOG_TOPICS,
});
const log = logger.log;
let rpc = createRpc({
  write: v => process.send(v),
  log,
});

process.on("message", async (message: RpcMessage) => {
  if (message.method === "exit") {
    exit(0);
  }

  if (typeof lsp === "undefined") {
    lsp = createLsp(
      utilTypes.convertEnv({
        envVar: name => process.env[name],
        now: () => new Date(),
        readFile: uri => {
          return uri.startsWith("file://") ?
            fs.readFile(fileURLToPath(uri)) :
            rpc.readFile(uri);
        },
        writeFile: (uri, content) => {
          return uri.startsWith("file://") ? 
            fs.writeFile(fileURLToPath(uri), content) :
            rpc.writeFile(uri, content);
        },
        stderr: process.stderr,
        stdErrAtty: () => process.stderr.isTTY,
        stdin: process.stdin,
        stdout: process.stdout,
        fetchFile: async (url: string) => {
          log("worker/fetchFile", url);
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
        rootUri: () => pathToFileURL(process.cwd()).toString(),
        fetch: {
          fetch,
          Headers,
          Request,
          Response,
        },
      }),
      {
        js_on_message: (message) => {
          log('lsp/worker2host', message);
          process.send(message);
        },
      }
    );
  }

  log('lsp/host2worker', message);
  if (!rpc.recv(message)) {
    lsp.send(message);
  }
});

// These are panics from Rust.
process.on("unhandledRejection", up => {
  throw up;
});
