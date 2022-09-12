const CREATE = "jsona-wasm-cli";
const CARGO_ARGS = "--features lsp"

const p = v => require("path").resolve(__dirname, v);
const $ = v => require("child_process").execSync(v, { stdio: "inherit" });
const fs = require("fs");
const cratePath = p(`../../crates/${CREATE}`);
$(`tsc`);
$(`wasm-pkg-build --out-name index --cargo-args "${CARGO_ARGS}" --modules esm-sync ${cratePath}`);
fs.copyFileSync(p("src/index_worker.d.ts"), p("dist/index_worker.d.ts"));
fs.copyFileSync(p(cratePath + "/pkg/index_worker.js"), p("dist/index_worker.js"));