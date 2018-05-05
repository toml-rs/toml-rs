//! ```
//! #[macro_use]
//! extern crate serde_derive;
//!
//! extern crate toml;
//! use toml::spanned::Spanned;
//!
//! #[derive(Deserialize)]
//! struct Udoprog {
//!     s: Spanned<String>,
//! }
//!
//! fn main() {
//!     let t = "s = \"udoprog\"\n";
//!
//!     let u: Udoprog = toml::from_str(t).unwrap();
//!
//!     assert_eq!(u.s.start, 4);
//!     assert_eq!(u.s.end, 13);
//! }
//! ```

use serde::{Serialize, Serializer};

// FIXME: use a more unique name like "toml::Spanned".
#[doc(hidden)]
pub const NAME: &str = "Spanned";
#[doc(hidden)]
pub const FIELDS: &[&str] = &["value", "start", "end"];

///
#[derive(Deserialize, Debug)]
pub struct Spanned<T> {
    ///
    pub value: T,
    ///
    pub start: usize,
    ///
    pub end: usize,
}

impl<T: Serialize> Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}
