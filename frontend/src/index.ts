import init, {
  FrameInputs,
  GameSettings,
  Instance,
  init_panic_hook,
} from "lib";

import { generateAuthToken } from "./auth";

import Alpine from "alpinejs";
import { initAlpine } from "./client";

declare global {
  interface Window {
    Alpine: typeof Alpine;
    backendUrl: string;
  }
}

await init({
  module_or_path: import.meta.env.DEV
    ? "lib_bg.wasm"
    : location.pathname + "/assets/lib_bg.wasm",
});

init_panic_hook();

window.backendUrl = "https://tetris.patzl.dev";
// window.backendUrl = "http://" + location.hostname + ":4444";

let running = false;

var then = window.performance.now();

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

const canvas = document.querySelector("canvas");
const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Canvas not found");
}
const game = new Instance(ctx, generateAuthToken, window.backendUrl);

const joinGame = async (gameId: string) => {
  if (running) {
    return;
  }
  pressedKeys.clear();
  try {
    console.log("Connecting to game ", gameId);
    game.connect(gameId).then(startGame);
  } catch (e) {
    console.error("Error connecting to game?: ", e);
  }
};

const runSinglePlayer = (settings: Pick<GameSettings, keyof GameSettings>) => {
  if (running) {
    return;
  }

  pressedKeys.clear();
  game.start_singleplayer(
    new GameSettings(
      settings.jupiter,
      settings.easy,
      settings.nes,
      settings.random
    )
  );
  startGame();
};

const stopEverything = () => {
  running = false;
  pressedKeys.clear();
  game.goodbye();
};

document.addEventListener("alpine:init", () => {
  initAlpine(joinGame, runSinglePlayer, stopEverything);
});

window.Alpine = Alpine;
Alpine.start();

// clear pressed keys before alerts since keyup events won't fire
globalThis.tetris_confirm = (text: string) => {
  pressedKeys.clear();
  return confirm(text);
};

globalThis.tetris_prompt = (text: string) => {
  pressedKeys.clear();
  return prompt(text);
};

declare global {
  interface ImportMeta {
    env: any;
  }
}

function startGame() {
  console.log("starting game");
  running = true;
  console.log("Running");
  then = window.performance.now();
  requestAnimationFrame(update);
}

async function update(newtime: number) {
  if (!running) {
    return;
  }
  let elapsed = newtime - then;
  then = newtime;
  if (elapsed > 500) {
    elapsed = 500;
  }

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
    if (!running) {
      console.log("Game ended");
    }
    elapsed -= fpsInterval;
  }
  if (running) requestAnimationFrame(update);

  then -= elapsed;
  game.draw();
}

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
