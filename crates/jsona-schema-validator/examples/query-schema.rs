use jsona::dom::{Keys, Node};
use jsona_schema_validator::JSONASchemaValidator;

fn main() {
    let mut args = std::env::args();
    let jsona_file = args
        .nth(1)
        .expect("Usage: query-schema <schema-jsona-file> [keys]");
    let keys = args.next();
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content =
        std::fs::read_to_string(jsona_file_path).expect("not found schema jsona file");
    let node: Node = jsona_content.parse().expect("invalid file");
    let validator = JSONASchemaValidator::try_from(&node).expect("invalid schema");
    let keys = match keys {
        Some(keys) => keys.parse().expect("invalid query path"),
        None => Keys::default(),
    };
    let result = serde_json::to_string_pretty(&validator.pointer(&keys)).unwrap();
    println!("{}", result);
}
