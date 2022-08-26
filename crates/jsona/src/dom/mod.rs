//! DOM(document object module) for JSONA
//!
//! DOM be constructed for data-oriented analysis where each node wraps a part of the
//! syntax tree with additional information and functionality.

#[macro_use]
mod macros;

pub mod error;
pub mod keys;
pub mod node;
pub mod query_keys;
pub mod visitor;

pub(crate) mod from_syntax;
mod serde;
mod to_string;

pub use error::*;
pub use from_syntax::from_syntax;
pub use keys::*;
pub use node::*;
pub use query_keys::*;
pub use visitor::*;
