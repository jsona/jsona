use jsona::{dom::Node, error::ErrorObject, util::mapper::Mapper};
use jsona_schema::Schema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
struct ParseResult {
    value: Option<Schema>,
    errors: Option<Vec<ErrorObject>>,
}

#[wasm_bindgen]
pub fn parse(input: &str) -> JsValue {
    let mapper = Mapper::new_utf16(input, false);
    let node = match Node::from_str(input) {
        Ok(v) => v,
        Err(err) => {
            return JsValue::from_serde(&ParseResult {
                value: None,
                errors: Some(err.to_error_objects(&mapper)),
            })
            .unwrap()
        }
    };
    let result = match Schema::try_from(&node) {
        Ok(v) => ParseResult {
            value: Some(v),
            errors: None,
        },
        Err(errs) => ParseResult {
            value: None,
            errors: Some(
                errs.into_iter()
                    .map(|v| v.to_error_object(&node, &mapper))
                    .collect(),
            ),
        },
    };
    JsValue::from_serde(&result).unwrap()
}
