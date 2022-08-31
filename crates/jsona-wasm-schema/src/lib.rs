use jsona::{dom::Node, util::mapper::Mapper};
use jsona_schema::Schema;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse(input: &str) -> Result<JsValue, JsValue> {
    let mapper = Mapper::new_utf16(input, false);
    let node = Node::from_str(input)
        .map_err(|err| JsValue::from_serde(&err.to_error_objects(&mapper)).unwrap())?;
    let schema = Schema::try_from(&node)
        .map_err(|err| JsValue::from_serde(&[err.to_error_object(&node, &mapper)]).unwrap())?;
    Ok(JsValue::from_serde(&schema).unwrap())
}
