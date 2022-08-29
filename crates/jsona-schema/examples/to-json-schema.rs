use jsona::dom::Keys;
use jsona_schema::Schema;
fn main() {
    let mut args = std::env::args();
    let jsona_file = args
        .nth(1)
        .expect("Usage: to-json-schema <jsona-file> [keys]");
    let keys = args.next();
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();
    let schema: Schema = jsona_content.parse().unwrap();
    let result = match keys {
        Some(keys) => {
            let keys: Keys = keys.parse().unwrap();
            serde_json::to_string_pretty(&schema.pointer(&keys)).unwrap()
        }
        None => serde_json::to_string_pretty(&schema).unwrap(),
    };
    println!("{}", result);
}
