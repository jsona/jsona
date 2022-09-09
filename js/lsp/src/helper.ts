
export function createRpc({ write, log }) {
  const rpc = {
    idx: -1,
    waitings: {},
    timeouts: {},
    async readFile(uri: string) {
      return rpc.send("lsp/readFile", { uri }).then(data => fromBase64(data)) as Promise<Uint8Array>;
    },
    async writeFile(uri: string, content: Uint8Array) {
      return rpc.send("lsp/writeFile", { uri, content: toBase64(content) }) as Promise<void>;
    },
    send<P = any, R = any>(method: string, params: P) {
      const id = rpc.idx--;
      const req = {
        jsonrpc: "2.0",
        id,
        method,
        params,
      }
      return new Promise<R>((resolveFn, rejectFn) => {
        const resolve = v => {
          rpc.clean(id);
          return resolveFn(v)
        }
        const reject = v => {
          rpc.clean(id);
          return rejectFn(v)
        }
        rpc.timeouts[id] = setTimeout(() => reject("Operation timeout"), 10000);
        rpc.waitings[id] = { resolve, reject }
        log('lsp/worker2host', req);
        write(req)
      })
    },
    recv(message) {
      if (typeof message.id === "number" && message.id < 0) {
        const wait = rpc.waitings[message.id];
        if (wait) {
          if (message?.error) {
            wait.reject(message.error?.message || "Unknown error")
          } else {
            wait.resolve(message?.result)
          }
        }
        return true
      }
      return false
    },
    clean(id) {
      delete rpc.waitings[id];
      clearTimeout(rpc.timeouts[id]);
      delete rpc.timeouts[id];
    },
  }
  return rpc;
}

export function toBase64(u8: ArrayBufferLike) {
  return btoa(String.fromCharCode.apply(null, u8));
}

export function fromBase64(str) {
  return atob(str).split('').map(function (c) { return c.charCodeAt(0); });
}

export function createLogger({ debug = false, topics = "" }) {
  const logger = {
    debug,
    topics: topics === "*" ? [""] : topics.split(",").map(v => v.trim()).filter(v => v.length > 0),
    level() {
      return this.debug ? "debug" : "info";
    },
    log(topic: string, message: any) {
      if (!logger.debug) return;
      if (!logger.topics.find(v => topic.startsWith(v))) return;
      let printMessage = message;
      if (typeof process !== "undefined" && typeof message !== "string") {
        printMessage = JSON.stringify(message);
      }
      if (message?.jsonrpc && message?.method) {
        console.log(topic, message.method, printMessage);
      } else {
        console.log(topic, printMessage);
      }
    }
  };
  return logger;
}
