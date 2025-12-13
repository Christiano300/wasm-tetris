import { defineConfig } from "vite";
import wasmPack from "vite-plugin-wasm-pack";
import devQRCode from "vite-plugin-dev-qrcode";

export default defineConfig({
  plugins: [wasmPack("./lib"), devQRCode()],
  build: {
    target: "esnext",
    minify: "esbuild",
  },
  assetsInclude: ["node_modules/lib/*.wasm", "assets/*.svg"],
  base: "/tetris",
});
