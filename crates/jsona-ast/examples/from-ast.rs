fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ast_file = args.get(1).expect("Usage: ast <ast-file>");
    let ast_file_path = std::path::Path::new(&ast_file);
    let ast_content = std::fs::read_to_string(ast_file_path).unwrap();
    let ast: jsona_ast::Ast = serde_json::from_str(&ast_content).unwrap();
    let node: jsona::dom::Node = ast.into();
    println!("{}", node);
}
