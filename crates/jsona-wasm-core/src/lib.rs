use gloo_utils::format::JsValueSerdeExt;
use jsona::dom::Node;
use jsona::error::ErrorObject;
use jsona::formatter::{self, Options};
use jsona_ast::{Ast, Mapper};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
struct ParseResult<T> {
    value: Option<T>,
    errors: Option<Vec<ErrorObject>>,
}

#[wasm_bindgen]
pub fn parse(input: &str) -> JsValue {
    let mapper = Mapper::new_utf16(input, false);
    let result = match input.parse::<Node>() {
        Ok(v) => ParseResult {
            value: Some(v.to_plain_json()),
            errors: None,
        },
        Err(error) => ParseResult {
            value: None,
            errors: Some(error.to_error_objects(&mapper)),
        },
    };
    JsValue::from_serde(&result).unwrap()
}

#[wasm_bindgen(js_name = parseAst)]
pub fn parse_ast(input: &str) -> JsValue {
    let result = match input.parse::<Ast>() {
        Ok(v) => ParseResult {
            value: Some(v),
            errors: None,
        },
        Err(errors) => ParseResult {
            value: None,
            errors: Some(errors),
        },
    };
    JsValue::from_serde(&result).unwrap()
}

#[wasm_bindgen(js_name = stringifyAst)]
pub fn stringify_ast(data: JsValue) -> Result<String, JsError> {
    let ast: Ast = data
        .into_serde()
        .map_err(|_| JsError::new("invalid jsona ast"))?;
    let node: Node = ast.into();
    Ok(format!("{}", node))
}

#[wasm_bindgen]
pub fn format(input: &str, format_options: JsValue) -> Result<String, JsError> {
    let mut options: Options = Options::default();
    options.update(
        format_options
            .into_serde()
            .map_err(|_| JsError::new("invalid format options"))?,
    );
    Ok(formatter::format(input, options))
}
