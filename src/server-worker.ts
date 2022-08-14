import {
  BrowserMessageReader,
  BrowserMessageWriter,
} from "vscode-languageserver-protocol/browser";

import { JsonaLsp, RpcMessage } from "@jsona/lsp";

const worker: Worker = self as any;

const writer = new BrowserMessageWriter(worker);
const reader = new BrowserMessageReader(worker);

let jsona: JsonaLsp;

let conn = {
  idx: 0,
  waitings: {} as Record<number, { resolve: (value) => any, reject: (reason?) => void }>,
  timeouts: {} as Record<number, any>,
  async readFile(fsPath: string) {
    let id = conn.idx - 1;
    let req: RpcMessage = { jsonrpc: "2.0", method: "fs/readFile", id, params: { fsPath }};
    return new Promise<Uint8Array>((resolveFn, rejectFn) => {
      const resolve = v => {
        conn.clean(id);
        return resolveFn(v)
      }
      const reject = v => {
        conn.clean(id);
        return rejectFn(v)
      }
      conn.timeouts[id] = setTimeout(() => reject("Operation timeout"), 10000);
      conn.waitings[id] = { resolve, reject  }
      writer.write(req)
    })
  },
  clean(id: number) {
    delete conn.waitings[id];
    clearTimeout(conn.timeouts[id]);
    delete conn.timeouts[id];
  }
}

reader.listen(async (message: RpcMessage) => {
  if (!jsona) {
    jsona = await JsonaLsp.init(
      {
        cwd: () => "/",
        envVar: (name) => {
          if (name === "RUST_LOG") {
            return import.meta.env.RUST_LOG;
          } else {
            return "";
          }
        },
        glob: () => [],
        now: () => new Date(),
        readFile: conn.readFile,
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
  if (typeof message.id === "number" && message.id < 0) {
    const wait = conn.waitings[message.id];
    if (wait) {
      if (message?.error) {
        wait.reject(message.error?.message || "Unknown error")
      } else {
        wait.resolve(message?.result)
      }
    }
  } else {
    jsona.send(message);
  }
});

function log(topic: "lsp2host" | "host2lsp" | "fetchFile", message: any) {
  if((import.meta.env.LOG_TOPICS).indexOf(topic) > -1) {
    if (message?.jsonrpc && message?.method)  {
      console.log(topic, message.method, message);
    } else {
      console.log(topic, message);
    }
  }
}