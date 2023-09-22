const CREATE = "jsona-wasm-core";

const fs = require("fs");
const p = v => require("path").resolve(__dirname, v);
const $ = v => require("child_process").execSync(v, { stdio: "inherit" });
const cratePath = p(`../../crates/${CREATE}`);

$(`wasm-pkg-build --out-name index ${cratePath}`);
[
  "index_bg.js",
  "index_bg.wasm",
  "index_bg.wasm.d.ts",
  "index_web.js",
  "index.js",
].forEach(name => {
  fs.copyFileSync(p(cratePath + "/pkg/" + name), p(name));
});