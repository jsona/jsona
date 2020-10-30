mod utils;

use jsona::Loader;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn parse(data: String) -> Result<JsValue, JsValue> {
    match Loader::load_from_str(data.as_str()) {
        Ok(ast) => Ok(JsValue::from_serde(&ast).unwrap()),
        Err(err) => Err(JsValue::from_serde(&err).unwrap()),
    }
}
