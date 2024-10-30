use wasm_bindgen::prelude::*;

mod draw;
mod game;
mod types;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, World!");
}
