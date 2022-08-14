use jsona::ast::Ast;
use jsona::dom::Node;
use jsona::formatter::{self, Options};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_ast(jsona: &str) -> Result<JsValue, JsValue> {
    match jsona.parse::<Ast>() {
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
