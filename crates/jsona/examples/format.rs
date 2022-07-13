use jsona::formatter::{format, Options};
fn main() {
    let jsona_file = std::env::args()
        .nth(1)
        .expect("Usage: print_syntax <jsona-file>");
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();

    let result = format(&jsona_content, Options::default());
    println!("{}", result);
}
