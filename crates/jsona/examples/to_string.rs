use jsona::dom::Node;
fn main() {
    let jsona_file = std::env::args()
        .nth(1)
        .expect("Usage: to_string <jsona-file>");
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();
    let node: Node = jsona_content.parse().unwrap();
    println!("{}", node);
}
