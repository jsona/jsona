use jsona::dom::Node;
use jsona_schema_validator::JSONASchemaValidator;

fn main() {
    let mut args = std::env::args();
    args.next();
    let (schema_jsona_path, plain_jsona_path) =
        if let (Some(v1), Some(v2)) = (args.next(), args.next()) {
            (v1, v2)
        } else {
            println!("Usage: validate <schema-jsona> <to-validate-jsona>");
            return;
        };
    let schema_jsona = std::fs::read_to_string(std::path::Path::new(&schema_jsona_path))
        .expect("not found schema jsona file");
    let schema_node: Node = schema_jsona.parse().expect("invalid schema jsona file");
    let validator =
        JSONASchemaValidator::from_node(&schema_node).expect("invalid schema jsona schema");
    let plain_jsona = std::fs::read_to_string(std::path::Path::new(&plain_jsona_path))
        .expect("not found to validate jsona file");
    let plain_node: Node = plain_jsona.parse().expect("invalid to validate jsona file");
    let errors = validator.validate(&plain_node);
    errors.iter().for_each(|err| {
        println!("{}", err);
    });
}
