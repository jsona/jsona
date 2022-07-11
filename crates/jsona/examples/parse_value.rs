fn main() {
    let jsona_file = std::env::args()
        .nth(1)
        .expect("Usage: print_syntax <jsona-file>");
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();

    let parse_result = jsona::parser::parse(&jsona_content);

    for err in &parse_result.errors {
        eprintln!("{}", err);
    }
    let syntax = parse_result.into_syntax();
    let node = jsona::dom::from_syntax(syntax.into());
    if let Err(errs) = node.validate() {
        for err in errs {
            eprintln!("{}", err);
        }
    }
    let value = jsona::value::Value::from(&node);
    println!("{:#?}", value);
}
