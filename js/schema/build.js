const CREATE = "jsona-wasm-schema";

const fs = require("fs");
const p = v => require("path").resolve(__dirname, v);
const $ = v => require("child_process").execSync(v, { stdio: "inherit" });
const cratePath = p(`../../crates/${CREATE}`);

$(`wasm-pack build --out-name index ${cratePath}`);
[
  "index_bg.js",
  "index_bg.wasm",
  "index_bg.wasm.d.ts",
].forEach(name => {
  fs.copyFileSync(p(cratePath + "/pkg/" + name), p(name));
});
$(`npx wasm-pack-utils node ${p("index_bg.js")} -o ${p("index.js")}`);
$(`npx wasm-pack-utils web ${p("index_bg.js")} -o ${p("index_web.js")}`);