import {
  BrowserMessageReader,
  BrowserMessageWriter,
} from "vscode-languageserver-protocol/browser";

import JsonaLsp, { RpcMessage } from "@jsona/lsp";

let debug = true;
const worker: Worker = self as any;

const writer = new BrowserMessageWriter(worker);
const reader = new BrowserMessageReader(worker);

let jsona: JsonaLsp;
let rootUri = "inmemory:///";

let com = {
  idx: -1,
  waitings: {} as Record<number, { resolve: (value) => any, reject: (reason?) => void }>,
  timeouts: {} as Record<number, any>,
  async readFile(uri: string) {
    return com.send({ method: "fs/readFile", params: { uri } });
  },
  send(req: Partial<RpcMessage>) {
    const id = com.idx--;
    req.jsonrpc = "2.0";
    req.id = id;
    return new Promise<Uint8Array>((resolveFn, rejectFn) => {
      const resolve = v => {
        com.clean(id);
        return resolveFn(v)
      }
      const reject = v => {
        com.clean(id);
        return rejectFn(v)
      }
      com.timeouts[id] = setTimeout(() => reject("Operation timeout"), 10000);
      com.waitings[id] = { resolve, reject }
      log('lsp2host', req);
      writer.write(req as RpcMessage)
    })
  },
  clean(id: number | string) {
    delete com.waitings[id];
    clearTimeout(com.timeouts[id]);
    delete com.timeouts[id];
  }
}

reader.listen(async (message: RpcMessage) => {
  if (!jsona) {
    jsona = await JsonaLsp.getInstance(
      {
        envVar: (name) => {
          if (name === "RUST_LOG") {
            return debug ? "debug" : "info";
          } else {
            return "";
          }
        },
        now: () => new Date(),
        readFile: com.readFile,
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
      },
      {
        onMessage(message: RpcMessage) {
          log('lsp2host', message);
          writer.write(message);
        },
      }
    );
  }

  log('host2lsp', message);
  if (typeof message.id === "number" && message.id < 0) {
    const wait = com.waitings[message.id];
    if (wait) {
      if (message?.error) {
        wait.reject(message.error?.message || "Unknown error")
      } else {
        wait.resolve(message?.result)
      }
    }
  } else if (message.method === "internal/setup") {
    debug = message.params.debug;
  } else {
    jsona.send(message);
  }
});

function log(topic: "lsp2host" | "host2lsp" | "fetchFile", message: any) {
  if (debug) {
    if (message?.jsonrpc && message?.method) {
      console.log(topic, message.method, message);
    } else {
      console.log(topic, message);
    }
  }
}