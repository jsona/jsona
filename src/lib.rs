mod utils;

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


#[wasm_bindgen]
pub fn parse(data: String) -> JsValue {
    match jsona::from_str(data.as_str()) {
        Ok(jsona) => JsValue::from_serde(&json!({"jsona": jsona})).unwrap(),
        Err(error) => JsValue::from_serde(&json!({"error": error})).unwrap(),
    }
}

#[wasm_bindgen(js_name = parseAsJSON)]
pub fn parse_as_json(data: String) -> JsValue {
    match jsona::from_str(data.as_str()) {
        Ok(jsona) => JsValue::from_serde(&json!({"jsona": Value::from(jsona)})).unwrap(),
        Err(error) => JsValue::from_serde(&json!({"error": error})).unwrap(),
    }
}
