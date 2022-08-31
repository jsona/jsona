use jsona::dom::Node;
use jsona::formatter::{self, Options};
use jsona_ast::{Ast, Mapper};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse(input: &str) -> Result<JsValue, JsValue> {
    let mapper = Mapper::new_utf16(input, false);
    match input.parse::<Node>() {
        Ok(node) => Ok(JsValue::from_serde(&node.to_plain_json()).unwrap()),
        Err(error) => Err(JsValue::from_serde(&error.to_error_objects(&mapper)).unwrap()),
    }
}

#[wasm_bindgen]
pub fn parse_ast(input: &str) -> Result<JsValue, JsValue> {
    match input.parse::<Ast>() {
        Ok(ast) => Ok(JsValue::from_serde(&ast).unwrap()),
        Err(error) => Err(JsValue::from_serde(&error).unwrap()),
    }
}

#[wasm_bindgen]
pub fn stringify_ast(data: JsValue) -> Result<String, JsError> {
    let ast: Ast = data
        .into_serde()
        .map_err(|_| JsError::new("invalid jsona ast"))?;
    let node: Node = ast.into();
    Ok(format!("{}", node))
}

#[wasm_bindgen]
pub fn format(jsona: &str, format_options: JsValue) -> Result<String, JsError> {
    let mut options: Options = Options::default();
    options.update(
        format_options
            .into_serde()
            .map_err(|_| JsError::new("invalid format options"))?,
    );
    Ok(formatter::format(jsona, options))
}
