import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import path from "path";
import process from "process";
import { minify } from "rollup-plugin-esbuild";
import typescript from "rollup-plugin-typescript2";

export default {
  input: {
    index: "src/index.ts",
  },
  output: {
    sourcemap: false,
    name: "jsonaLsp",
    format: "umd",
    dir: "dist",
  },
  plugins: [
    typescript(),
    rust({
      inlineWasm: true,
      cargoArgs: ["-F", "lsp"],
    }),
    commonjs(),
    resolve({
      jsnext: true,
      preferBuiltins: true,
      rootDir: path.join(process.cwd(), ".."),
    }),
    minify(),
  ],
};
