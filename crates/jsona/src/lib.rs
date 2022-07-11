pub mod dom;
pub mod parser;
pub mod syntax;
pub mod util;
pub mod value;

mod private {
    pub trait Sealed {}
}
