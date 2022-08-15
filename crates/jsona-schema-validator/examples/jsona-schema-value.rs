use jsona::dom::{Keys, Node};
use jsona_schema_validator::JSONASchemaValue;

fn main() {
    let mut args = std::env::args();
    let jsona_file = args
        .nth(1)
        .expect("Usage: to-json-schema <jsona-file> [keys]");
    let keys = args.next();
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();
    let node: Node = jsona_content.parse().expect("invalid jsona doc");
    let schema_value = JSONASchemaValue::from_node(node).expect("invalid jsona schema value");
    let keys: Keys = match keys {
        Some(keys) => keys.parse().unwrap(),
        None => Keys::default(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&schema_value.pointer(&keys)).unwrap()
    );
}
