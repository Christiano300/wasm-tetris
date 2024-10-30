import init, { Action, Game } from "lib";

await init("/assets/lib_bg.wasm");

let actions = [] as Action[];
var downPressed = false;

function update() {
  window.requestAnimationFrame(update);

  if (downPressed) {
    actions.push(Action.SoftDrop);
  }
  game.update(actions);
  game.draw();
  actions.splice(0, actions.length);
}

const canvas = document.querySelector("canvas");
const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Canvas not found");
}
const game = new Game(ctx);

window.addEventListener("keydown", (e) => {
  if (e.key === "ArrowLeft") {
    actions.push(Action.Left);
  } else if (e.key === "ArrowRight") {
    actions.push(Action.Right);
  } else if (e.key === "ArrowUp") {
    actions.push(Action.Cw);
  } else if (e.key === "ArrowDown") {
    downPressed = true;
  } else if (e.key === " ") {
    actions.push(Action.HardDrop);
  } else if (e.key === "c") {
    actions.push(Action.Hold);
  } else if (e.code === "KeyZ") {
    actions.push(Action.Ccw);
  }
});

window.addEventListener("keyup", (e) => {
  if (e.key === "ArrowDown") {
    downPressed = false;
  }
});

update();