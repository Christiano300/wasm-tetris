import init, { FrameInputs, Instance, init_panic_hook } from "lib";

import { generateAuthToken } from "./auth";

// clear pressed keys before alerts since keyup events won't fire
globalThis.tetris_confirm = (text: string) => {
  pressedKeys.clear();
  return confirm(text);
};

globalThis.tetris_prompt = (text: string) => {
  pressedKeys.clear();
  return prompt(text);
};

await init();

init_panic_hook();

const pressedKeys = new Set();
const keyMap = {
  ArrowLeft: "left",
  ArrowRight: "right",
  ArrowUp: "cw",
  ArrowDown: "soft_drop",
  " ": "hard_drop",
  c: "hold",
  z: "ccw",
  a: "left",
  d: "right",
  s: "soft_drop",
  j: "ccw",
  l: "cw",
  k: "hold",
};
const controls = [
  "left",
  "right",
  "cw",
  "ccw",
  "hold",
  "hard_drop",
  "soft_drop",
];
const fpsInterval = 1000 / 60;

let running = false;

var then = window.performance.now();

function startGame() {
  running = true;
  then = window.performance.now();
  requestAnimationFrame(update);
}

async function update(newtime: number) {
  let elapsed = newtime - then;
  
  const keys = controls.map((key) => pressedKeys.has(key)) as [
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean,
    boolean
  ];
  while (elapsed > fpsInterval) {
    running = await game.update(new FrameInputs(...keys));
    elapsed -= fpsInterval;
  }
  if (running) requestAnimationFrame(update);

  then = newtime - elapsed;
  game.draw();
}

const canvas = document.querySelector("canvas");
const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Canvas not found");
}
const game = new Instance(ctx, generateAuthToken);

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

document.querySelector("button")?.addEventListener("click", async () => {
  if (running) {
    return;
  }
  pressedKeys.clear();
  try {
    game
      .connect((document.getElementById("gameId") as HTMLInputElement).value)
      .then(startGame);
  } catch (e) {
    console.error("Error connecting to game?: ", e);
  }
});
