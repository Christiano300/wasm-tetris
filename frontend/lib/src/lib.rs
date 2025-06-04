use wasm_bindgen::prelude::wasm_bindgen;

mod draw;
mod input;
mod instance;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = globalThis)]
    fn tetris_confirm(message: &str) -> bool;

    #[wasm_bindgen(js_namespace = globalThis)]
    fn tetris_prompt(message: &str) -> Option<String>;
}
