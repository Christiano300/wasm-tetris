import { defineConfig } from "vite";
import wasmPack from "vite-plugin-wasm-pack";

export default defineConfig({
  plugins: [wasmPack("./lib")],
  build: {
    target: "esnext",
    minify: "esbuild",
  },
  assetsInclude: ["node_modules/lib/*.wasm", "assets/*.svg"],
  base: "/tetris"
});
