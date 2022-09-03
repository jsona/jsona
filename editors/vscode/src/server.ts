import fsPromise from "fs/promises";
import { exit } from "process";
import { pathToFileURL } from "url";
import JsonaLsp, { RpcMessage} from "@jsona/lsp";
import fetch, { Headers, Request, Response } from "node-fetch";

let jsona: JsonaLsp;

process.on("message", async (message: RpcMessage) => {
  if (message.method === "exit") {
    exit(0);
  }

  if (typeof jsona === "undefined") {
    jsona = await JsonaLsp.getInstance(
      {
        envVar: name => process.env[name],
        now: () => new Date(),
        readFile: path => fsPromise.readFile(path),
        writeFile: (path, content) => fsPromise.writeFile(path, content),
        stderr: process.stderr,
        stdErrAtty: () => process.stderr.isTTY,
        stdin: process.stdin,
        stdout: process.stdout,
        fetchFile: async (url: string) => {
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
        rootUri: () => pathToFileURL(process.cwd()).toString(),
        fetch: {
          fetch,
          Headers,
          Request,
          Response,
        },
      },
      {
        onMessage(message) {
          log('lsp2host', message);
          process.send(message);
        },
      }
    );
  }

  log('host2lsp', message);
  jsona.send(message);
});

// These are panics from Rust.
process.on("unhandledRejection", up => {
  throw up;
});

function log(topic: "lsp2host" | "host2lsp" | "fetchFile", message: any) {
  if((import.meta.env.LOG_TOPICS).indexOf(topic) > -1) {
    if (typeof message === "object") {
      console.log(topic, JSON.stringify(message));
      if (message?.jsonrpc && message?.method)  {
        console.log(topic, message.method, message);
      } else {
        console.log(topic, message);
      }
    } else {
      console.log(topic, message);
    }
  }
}