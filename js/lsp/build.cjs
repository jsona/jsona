const CREATE = "jsona-wasm-cli";
const CARGO_ARGS = "--features lsp"

const p = v => require("path").resolve(__dirname, v);
const $ = v => require("child_process").execSync(v, { stdio: "inherit" });
const fs = require("fs");
const cratePath = p(`../../crates/${CREATE}`);
$(`tsc`);
fs.copyFileSync(p("src/index_worker.d.ts"), p("dist/index_worker.d.ts"));
$(`wasm-pack build --out-name index ${cratePath} ${CARGO_ARGS}`);
$(`npx wasm-pack-utils worker ${p(cratePath + "/pkg/index_bg.js")} --inline-wasm -o ${p("dist/index_worker.js")}`);