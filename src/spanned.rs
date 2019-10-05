//! Data structures for spanned data
use serde::{de, ser};
use std::fmt;

/// We map Spanned<T> to a special valid in the serde data model.
///
/// In general the TOML encoder/decoder will catch this and not literally emit
/// these strings but rather emit datetimes as they're intended.
///
/// The constants are exposed to allow users of this crate to use these
/// constants during serialization and deserialization.
///
/// The value of these constants may change with any toml version change.
pub const NAME: &str = "$__toml_private_Spanned";
/// The first field of a Spanned<T>. For more, see the `NAME` constant.
pub const START: &str = "$__toml_private_start";
/// The second field of a Spanned<T>. For more, see the `NAME` constant.
pub const END: &str = "$__toml_private_end";
/// The third field of a Spanned<T>. For more, see the `NAME` constant.
pub const VALUE: &str = "$__toml_private_value";

/// A spanned value, indicating the range at which it is defined in the source.
///
/// ```
/// use serde_derive::Deserialize;
/// use toml::Spanned;
///
/// #[derive(Deserialize)]
/// struct Value {
///     s: Spanned<String>,
/// }
///
/// fn main() {
///     let t = "s = \"value\"\n";
///
///     let u: Value = toml::from_str(t).unwrap();
///
///     assert_eq!(u.s.start(), 4);
///     assert_eq!(u.s.end(), 11);
///     assert_eq!(u.s.get_ref(), "value");
///     assert_eq!(u.s.into_inner(), String::from("value"));
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    /// The start range.
    start: usize,
    /// The end range (exclusive).
    end: usize,
    /// The spanned value.
    value: T,
}

impl<T> Spanned<T> {
    /// Access the start of the span of the contained value.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Access the end of the span of the contained value.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Get the span of the contained value.
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Consumes the spanned value and returns the contained value.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Returns a reference to the contained value.
    pub fn get_ref(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the contained value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'de, T> de::Deserialize<'de> for Spanned<T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Spanned<T>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct SpannedVisitor<T>(::std::marker::PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for SpannedVisitor<T>
        where
            T: de::Deserialize<'de>,
        {
            type Value = Spanned<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a TOML spanned")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Spanned<T>, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                if visitor.next_key()? != Some(START) {
                    return Err(de::Error::custom("spanned start key not found"));
                }

                let start: usize = visitor.next_value()?;

                if visitor.next_key()? != Some(END) {
                    return Err(de::Error::custom("spanned end key not found"));
                }

                let end: usize = visitor.next_value()?;

                if visitor.next_key()? != Some(VALUE) {
                    return Err(de::Error::custom("spanned value key not found"));
                }

                let value: T = visitor.next_value()?;

                Ok(Spanned { start, end, value })
            }
        }

        let visitor = SpannedVisitor(::std::marker::PhantomData);

        static FIELDS: [&str; 3] = [START, END, VALUE];
        deserializer.deserialize_struct(NAME, &FIELDS, visitor)
    }
}

impl<T: ser::Serialize> ser::Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.value.serialize(serializer)
    }
}
