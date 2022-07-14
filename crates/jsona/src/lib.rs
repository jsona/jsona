#![allow(clippy::single_match)]
//! # About
//!
//! The main purpose of the library is to provide tools for analyzing JSONA data where the
//! layout must be preserved and the original position of every parsed token must be known. It can
//! also format JSONA documents.
//!
//! It uses [Rowan](::rowan) for the syntax tree, and every character is preserved from the input,
//! including all comments and white space.
//!
//! A [DOM](dom) can be constructed for data-oriented analysis where each node wraps a part of the
//! syntax tree with additional information and functionality.
//!
//! # Features
//!
//! - **serde**: Support for [serde](https://serde.rs) serialization of the DOM nodes.
//!
//! # Usage
//!
//! A JSONA document has to be parsed with [parse](parser::parse) first, it
//! will build a syntax tree that can be traversed.
//!
//! If there were no syntax errors during parsing, then a [`dom::Node`]
//! can be constructed. It will build a DOM tree and validate the TOML document according
//! to the specification. A DOM tree can be constructed even with syntax errors present, however
//! parts of it might be missing.
//!
//! ```
//! use taplo::parser::parse;
//! const SOURCE: &str = r#"
//! {
//!   createPost: { @describe("Create a blog post") @mixin(["createPost", "auth1"])
//!     req: {
//!       body: {
//!         content: "paragraph", @mock
//!       }
//!     },
//!     res: {
//!       body: {
//!         id: 0, @type
//!         userId: 0, @type
//!         content: "", @type
//!       }
//!     }
//!   }
//! }
//! "#;
//!
//! let parse_result = parse(SOURCE);
//!
//! // Check for syntax errors.
//! // These are not carried over to DOM errors.
//! assert!(parse_result.errors.is_empty());
//!
//! let root_node = parse_result.into_dom();
//!
//! // Check for semantic errors.
//! // In this example "value" is a duplicate key.
//! assert!(root_node.validate().is_err());
//! ```

pub mod dom;
pub mod formatter;
pub mod parser;
pub mod syntax;
pub mod util;
pub mod value;

pub use rowan;

mod private {
    pub trait Sealed {}
}
