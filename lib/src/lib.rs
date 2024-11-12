use wasm_bindgen::prelude::wasm_bindgen;

mod draw;
mod game;
mod types;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn alert(s: &str);
}
