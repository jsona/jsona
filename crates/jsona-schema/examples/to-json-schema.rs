use jsona_schema::from_str;
fn main() {
    let jsona_file = std::env::args()
        .nth(1)
        .expect("Usage: to-json-schema <jsona-file>");

    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();
    let schema = from_str(&jsona_content).unwrap();
    let result = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", result);
}
