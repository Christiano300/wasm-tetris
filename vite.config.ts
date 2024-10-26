import { defineConfig } from "vite";
import wasmPack from "vite-plugin-wasm-pack";

export default defineConfig({
  plugins: [wasmPack("./lib")],
  build: {
    target: "esnext",
    minify: "esbuild",
  },
  resolve: {dedupe: ["vscode"]},
  assetsInclude: "node_modules/lib/*.wasm"
});
