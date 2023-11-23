import commonjs from "@rollup/plugin-commonjs";
import resolve from "@rollup/plugin-node-resolve";
import esbuild from "rollup-plugin-esbuild";


/** @type {import('rollup').RollupOptions} */
const options = {
  input: {
    "jsona.worker": "src/jsona.worker.ts",
  },
  output: {
    sourcemap: false,
    format: "umd",
    dir: "./",
    chunkFileNames: "[name].js",
  },
  external: ["vscode"],
  preserveEntrySignatures: true,
  treeshake: "smallest",
  plugins: [
    esbuild({ minify: true, logLevel: "error" }),
    commonjs({
      ignore: ["url"],
    }),
    resolve({
      preferBuiltins: true,
      browser: true,
    }),
  ],
};

export default options;
