import fs from "fs";
import fsPromise from "fs/promises";
import path from "path";
import { exit } from "process";
import { RpcMessage, JsonaLsp } from "@jsona/lsp";
import fetch, { Headers, Request, Response } from "node-fetch";
import glob from "fast-glob";

let jsona: JsonaLsp;

process.on("message", async (d: RpcMessage) => {
  if (d.method === "exit") {
    exit(0);
  }

  if (typeof jsona === "undefined") {
    jsona = await JsonaLsp.init(
      {
        cwd: () => process.cwd(),
        envVar: name => process.env[name],
        findConfigFile: from => {
          const fileNames = [".jsona"];

          for (const name of fileNames) {
            try {
              const fullPath = path.join(from, name);
              fs.accessSync(fullPath);
              return fullPath;
            } catch {}
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
          let value = decodeURI(url).slice("file://".length);
          if (path.sep == "\\") {
            return value.replace(/\//g, "\\");
          }
          return value;
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
          process.send(message);
        },
      }
    );
  }

  jsona.send(d);
});

// These are panics from Rust.
process.on("unhandledRejection", up => {
  throw up;
});
