fn main() {
    let args: Vec<String> = std::env::args().collect();
    let jsona_file = args
        .get(1)
        .expect("Usage: value <jsona-file> [json|plain|jsona]");
    let output_format = args.get(2).map(|v| v.to_string()).unwrap_or_default();
    let jsona_file_path = std::path::Path::new(&jsona_file);
    let jsona_content = std::fs::read_to_string(jsona_file_path).unwrap();

    let parse_result = jsona::parser::parse(&jsona_content);

    for err in &parse_result.errors {
        eprintln!("{}", err);
    }
    let node = parse_result.into_dom();
    if let Err(errs) = node.validate() {
        for err in errs {
            eprintln!("{}", err);
        }
    }
    let output = match output_format.as_str() {
        "plain" => serde_json::to_string_pretty(&node).unwrap(),
        "jsona" => node.to_string(),
        _ => serde_json::to_string_pretty(&node.to_json()).unwrap(),
    };
    println!("{}", output);
}
