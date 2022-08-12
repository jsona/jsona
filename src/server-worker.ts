import {
  BrowserMessageReader,
  BrowserMessageWriter,
} from "vscode-languageserver-protocol/browser";

import { JsonaLsp, RpcMessage } from "@jsona/lsp";

const worker: Worker = self as any;

const writer = new BrowserMessageWriter(worker);
const reader = new BrowserMessageReader(worker);

let jsona: JsonaLsp;

reader.listen(async message => {
  if (!jsona) {
    jsona = await JsonaLsp.init(
      {
        cwd: () => "/",
        envVar: (name) => {
          if (name === "RUST_LOG") {
            return import.meta.env.RUST_LOG;
          }
          return "";
        },
        glob: () => [],
        isAbsolute: () => true,
        now: () => new Date(),
        readFile: () => Promise.reject("not implemented read_file "),
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
        urlToFilePath: (url: string) => url.slice("file://".length),
      },
      {
        onMessage(message) {
          log('lsp2host', message);
          writer.write(message);
        },
      }
    );
  }

  log('host2lsp', message);
  jsona.send(message as RpcMessage);
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