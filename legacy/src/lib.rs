pub mod syntax;
pub mod error;
pub mod lexer;
pub mod loader;
pub mod parser;

pub use syntax::Jsona;
pub use error::Error;

pub fn from_str(input: &str) -> Result<Jsona, Error> {
    loader::Loader::load_from_str(input)
}
