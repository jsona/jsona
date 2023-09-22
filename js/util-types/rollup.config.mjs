import typescript from '@rollup/plugin-typescript';
const config = [
  {
    input: 'src/index.ts',
    output: {
      dir: 'dist',
      format: 'cjs',
    },
    plugins: [typescript()]
  }, 
  {
    input: 'src/index.ts',
    output: {
      file: "dist/index.mjs",
      format: 'es'
    },
    plugins: [typescript()]
  }
];
export default config;