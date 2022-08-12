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
        findConfigFile: async from => {
          while (true) {
            try {
              const fullPath = path.join(from, ".jsona");
              await fsPromise.access(fullPath);
              return fullPath;
            } catch {}
            let from_ = path.resolve(from, "..");
            if (from_ === from) {
              return;
            }
            from = from_
          }
        },
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
          const controller = new AbortController();
          const timeout = setTimeout(() => {
            controller.abort();
          }, 10000);
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
          debugLog('lsp2host', message);
          process.send(message);
        },
      }
    );
  }

  debugLog('host2lsp', message);
  jsona.send(message);
});

// These are panics from Rust.
process.on("unhandledRejection", up => {
  throw up;
});

function debugLog(topic: string, message: any) {
  if (import.meta.env.RUST_LOG === "debug" || import.meta.env.RUST_LOG == "verbose") {
    console.log(topic, JSON.stringify(message));
  }
}