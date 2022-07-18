use jsona::syntax::stringify_syntax;

fn main() {
    let jsona_file = std::env::args()
        .nth(1)
        .expect("Usage: syntax <jsona-file>");
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();

    let parse_result = jsona::parser::parse(&jsona_content);

    for err in &parse_result.errors {
        eprintln!("{}", err);
    }
    println!(
        "{}",
        stringify_syntax(0, parse_result.into_syntax().into()).unwrap()
    )
}
