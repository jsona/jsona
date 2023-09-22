import commonjs from "@rollup/plugin-commonjs";
import resolve from "@rollup/plugin-node-resolve";
import esbuild from "rollup-plugin-esbuild";
import replace from "@rollup/plugin-replace";

/** @type {import('rollup').RollupOptions} */
const options = {
  input: {
    server: "src/server.ts",
    extension: "src/extension.ts",
  },
  output: {
    sourcemap: !!process.env.DEBUG,
    format: "commonjs",
    dir: "dist",
    chunkFileNames: "[name].js",
  },
  external: ["vscode"],
  preserveEntrySignatures: true,
  treeshake: "smallest",
  plugins: [
    replace({
      preventAssignment: true,
      "import.meta.env.BROWSER": false,
      "import.meta.env.DEBUG": !!process.env.DEBUG,
      "import.meta.env.LOG_TOPICS": JSON.stringify(process.env.LOG_TOPICS || ""),
    }),
    esbuild({ minify: !process.env.DEBUG }),
    commonjs(),
    resolve({
      preferBuiltins: true,
    }),
  ],
};

export default options;
