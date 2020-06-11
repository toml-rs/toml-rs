//! Low-level [TOML] lexer (highly inspired by [`rustc_lexer`])
//!
//! The idea with `toml_lexer` is to make a reusable library,
//! by separating out pure lexing and toml-parser-specific concerns, like
//! error reporting and high-level tokens interpretation
//! (e.g. deserializing with [`serde`] or making a language server).
//! So, `toml_lexer` operates directly on `&str` and produces simple [`Span`]
//! and [`TokenKind`] pairs.
//!
//! It never fails and doesn't even have any `Error` type, instead each [`Token`]
//! is possibly malformed. You can check this via the flags stored within the [`Token`] itself.
//!
//! Main entities of this crate are [`Tokenizer`], [`Token`], and [`TokenKind`].
//!
//! ```
//! # use toml_lexer::*;
//! let tokenizer = Tokenizer::new(r#"foo = |"bar\uf4O"#);
//! let token_kinds: Vec<TokenKind> = tokenizer
//!     .map(|Token { span: Span { start: _, end: _ }, kind }| kind)
//!     .collect();
//!
//! assert_eq!(token_kinds, vec![
//!     TokenKind::Keylike,
//!     TokenKind::Whitespace,
//!     TokenKind::Equals,
//!     TokenKind::Whitespace,
//!     TokenKind::Unknown,
//!     TokenKind::StrLitSubtoken(
//!         StrLitSubtoken::LeadingQuotes(
//!             Quotes { len: QuotesLen::X1, kind: StrLitKind::Basic }
//!         )
//!     ),
//!     TokenKind::StrLitSubtoken(StrLitSubtoken::Char('b')),
//!     TokenKind::StrLitSubtoken(StrLitSubtoken::Char('a')),
//!     TokenKind::StrLitSubtoken(StrLitSubtoken::Char('r')),
//!     TokenKind::StrLitSubtoken(
//!         StrLitSubtoken::UnicodeEscape {
//!             kind: HexLen::X4,
//!             unescaped: UnicodeEscape::NotEnoughDigits(2),
//!         }
//!     ),
//!     TokenKind::StrLitSubtoken(StrLitSubtoken::Char('O')),
//! ]);
//! ```
//!
//! [`Tokenizer`]: struct.Tokenizer.html
//! [`Token`]: struct.Token.html
//! [`TokenKind`]: enum.TokenKind.html
//! [`Span`]: struct.Span.html
//!
//! [TOML]: https://github.com/toml-lang/toml
//! [Cargo]: https://crates.io/
//! [`serde`]: https://serde.rs/
//! [`rustc_lexer`]: https://docs.rs/rustc-ap-rustc_lexer/

#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
// Makes rustc abort compilation if there are any unsafe blocks in the crate.
// Presence of this annotation is picked up by tools such as cargo-geiger
// and lets them ensure that there is indeed no unsafe code as opposed to
// something they couldn't detect (e.g. unsafe added via macro expansion, etc).
#![forbid(unsafe_code)]

mod cursor;
mod lexer;
mod str_lit;

pub use lexer::*;
pub use str_lit::*;

/// A span, designating a range of bytes where a token is located.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Span {
    /// The start of the range.
    pub start: usize,
    /// The end of the range (exclusive).
    pub end: usize,
}

impl From<Span> for (usize, usize) {
    fn from(Span { start, end }: Span) -> (usize, usize) {
        (start, end)
    }
}
