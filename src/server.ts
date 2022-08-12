import fsPromise from "fs/promises";
import path from "path";
import { exit } from "process";
import { RpcMessage, JsonaLsp } from "@jsona/lsp";
import fetch, { Headers, Request, Response } from "node-fetch";
import glob from "fast-glob";

let jsona: JsonaLsp;

process.on("message", async (message: RpcMessage) => {
  if (message.method === "exit") {
    exit(0);
  }

  if (typeof jsona === "undefined") {
    jsona = await JsonaLsp.init(
      {
        cwd: () => process.cwd(),
        envVar: name => process.env[name],
        glob: p => glob.sync(p),
        isAbsolute: p => path.isAbsolute(p),
        now: () => new Date(),
        readFile: path => fsPromise.readFile(path),
        writeFile: (path, content) => fsPromise.writeFile(path, content),
        stderr: process.stderr,
        stdErrAtty: () => process.stderr.isTTY,
        stdin: process.stdin,
        stdout: process.stdout,
        urlToFilePath: (url: string) => {
          url = decodeURIComponent(url);
          if (path.sep == "\\") {
            let value = url.slice("file:///".length);
            return value.replace(/\//g, "\\");
          } else {
            return url.slice("file://".length)
          }
        },
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
    } else {
      console.log(topic, message);
    }
  }
}