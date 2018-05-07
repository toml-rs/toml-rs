extern crate serde;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use toml::Spanned;
use std::collections::HashMap;

#[test]
fn test_spanned_field() {
    #[derive(Deserialize)]
    struct Foo<T> {
        foo: Spanned<T>,
    }

    fn good<'de, T>(s: &'de str, expected: &str) where T: serde::Deserialize<'de> {
        let foo: Foo<T> = toml::from_str(s).unwrap();

        assert_eq!(6, foo.foo.start);
        assert_eq!(s.len(), foo.foo.end);
        assert_eq!(expected, &s[foo.foo.start..foo.foo.end]);
    }

    good::<String>("foo = \"foo\"", "\"foo\"");
    good::<u32>("foo = 42", "42");
    good::<HashMap<String, u32>>(
        "foo = {\"foo\" = 42, \"bar\" = 42}",
        "{\"foo\" = 42, \"bar\" = 42}"
    );
}
