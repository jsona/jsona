#![allow(unused)]
use jsona::error::ErrorObject;

pub(crate) fn include_fixtures(file: &str) -> String {
    let mut file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file_path.pop();
    for p in format!("fixtures/{}", file).split('/') {
        file_path = file_path.join(p);
    }

    println!("path {}", file_path.display());
    std::fs::read_to_string(&file_path).unwrap()
}
