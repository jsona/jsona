use jsona::{
    parser::parse,
    value::{PlainValue, Value},
};

pub fn json_to_jsona(json: &str) -> Result<String, anyhow::Error> {
    let root: Value = serde_json::from_str(json)?;
    Ok(root.to_jsona())
}

pub fn jsona_to_json(toml: &str, with_annotation: bool) -> Result<String, anyhow::Error> {
    let root = parse(toml).into_dom();
    if with_annotation {
        let value = Value::from(&root);
        Ok(serde_json::to_string_pretty(&value)?)
    } else {
        let value = PlainValue::from(&root);
        Ok(serde_json::to_string_pretty(&value)?)
    }
}
