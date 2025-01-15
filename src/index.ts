import init, { Action, FrameInputs, Game, init_panic_hook } from "lib";

import { generateAuthToken } from "./auth";

await init({
  module_or_path: "/assets/lib_bg.wasm",
});

init_panic_hook();

let actions = [] as Action[];
const pressedKeys = new Set();
const keyMap = {ArrowLeft: "left", ArrowRight: "right", ArrowUp: "cw", ArrowDown: "soft_drop", " ": "hard_drop", c: "hold", KeyZ: "ccw"};
const frameInputs = ["left", "right", "cw", "ccw", "hold", "hard_drop", "soft_drop"];

function update() {
  window.requestAnimationFrame(update);

  const keys = frameInputs.map((key) => pressedKeys.has(key)) as [boolean, boolean, boolean, boolean, boolean, boolean, boolean];
  game.update(new FrameInputs(...keys));
  game.draw();
  actions.splice(0, actions.length);
}

const canvas = document.querySelector("canvas");
const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Canvas not found");
}
const game = new Game(ctx, generateAuthToken);

window.addEventListener("keydown", (e) => {
  const action = keyMap[e.key];
  if (action) {
    pressedKeys.add(action);
  }
});

window.addEventListener("keyup", (e) => {
  const action = keyMap[e.key];
  if (action) {
    pressedKeys.delete(action);
  }
});

update();