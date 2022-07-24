fn main() {
    let args: Vec<String> = std::env::args().collect();
    let jsona_file = args.get(1).expect("Usage: ast <jsona-file>");
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();

    match jsona_content.parse::<jsona::ast::Ast>() {
        Ok(ast) => {
            println!("{}", serde_json::to_string_pretty(&ast).unwrap());
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{:?}", error);
            }
        }
    }
}
