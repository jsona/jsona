mod utils;

use jsona::Loader;
use serde_json::Value;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(js_name = parseAst)]
pub fn parse_ast(data: String) -> Result<JsValue, JsValue> {
    match Loader::load_from_str(data.as_str()) {
        Ok(ast) => Ok(JsValue::from_serde(&ast).unwrap()),
        Err(err) => Err(JsValue::from_serde(&err).unwrap()),
    }
}

#[wasm_bindgen(js_name = parseJson)]
pub fn parse_json(data: String) -> Result<JsValue, JsValue> {
    match Loader::load_from_str(data.as_str()) {
        Ok(ast) => Ok(JsValue::from_serde(&Value::from(ast)).unwrap()),
        Err(err) => Err(JsValue::from_serde(&err).unwrap()),
    }
}
