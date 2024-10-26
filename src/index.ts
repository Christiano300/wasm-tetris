import init, {greet} from "lib";

await init("/assets/lib_bg.wasm");

document.querySelector("button")?.addEventListener("click", () => greet())