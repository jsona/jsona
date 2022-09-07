
export function createRpc({ write, log }) {
  const rpc = {
    idx: -1,
    waitings: {},
    timeouts: {},
    async readFile(uri: string) {
      return rpc.send({ method: "lsp/readFile", params: { uri } }) as Promise<Uint8Array>;
    },
    async writeFile(uri: string, content: Uint8Array) {
      return rpc.send({ method: "lsp/writeFile", params: { uri, content } }) as Promise<void>;
    },
    send(req) {
      const id = rpc.idx--;
      req.jsonrpc = "2.0";
      req.id = id;
      return new Promise((resolveFn, rejectFn) => {
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
        log('lsp2host', req);
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


export function createLogger({ debug, topics }) {
  const logger = {
    debug,
    topics,
    level() {
      return this.debug ? "debug" : "info";
    },
    log(topic, message) {
      if (!logger.debug) return;
      if (topics.indexOf(topic) === -1 && topics !== "*") return; 
      let printMessage = message;
      if (typeof process !== "undefined") printMessage = JSON.stringify(message);
      if (message?.jsonrpc && message?.method) {
        console.log(topic, message.method, printMessage);
      } else {
        console.log(topic, printMessage);
      }
    }
  };
  return logger;
}
