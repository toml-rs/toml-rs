extern crate toml;

use std::collections::HashMap;
use std::str::FromStr;
use toml::spanned_value::ValueKind as K;
use toml::value::Datetime;
use toml::SpannedValue;

/// A set of good datetimes.
pub fn good_datetimes() -> Vec<&'static str> {
    let mut v = Vec::new();
    v.push("1997-09-09T09:09:09Z");
    v.push("1997-09-09T09:09:09+09:09");
    v.push("1997-09-09T09:09:09-09:09");
    v.push("1997-09-09T09:09:09");
    v.push("1997-09-09");
    v.push("09:09:09");
    v.push("1997-09-09T09:09:09.09Z");
    v.push("1997-09-09T09:09:09.09+09:09");
    v.push("1997-09-09T09:09:09.09-09:09");
    v.push("1997-09-09T09:09:09.09");
    v.push("09:09:09.09");
    v
}

#[test]
fn test_spanned_field() {
    fn good<'de, T>(s: &'de str, expected: &str)
    where
        T: serde::Deserialize<'de>,
    {
        let foo: SpannedValue = toml::from_str(s).unwrap();

        let foo = &foo.get_ref()["foo"];

        assert_eq!(6, foo.start());
        assert_eq!(s.len(), foo.end());
        assert_eq!(expected, &s[foo.start()..foo.end()]);
    }

    good::<String>("foo = \"foo\"", "\"foo\"");
    good::<u32>("foo = 42", "42");
    // leading plus
    good::<u32>("foo = +42", "+42");
    // table
    good::<HashMap<String, u32>>(
        "foo = {\"foo\" = 42, \"bar\" = 42}",
        "{\"foo\" = 42, \"bar\" = 42}",
    );
    // array
    good::<Vec<u32>>("foo = [0, 1, 2, 3, 4]", "[0, 1, 2, 3, 4]");
    // datetime
    good::<String>("foo = \"1997-09-09T09:09:09Z\"", "\"1997-09-09T09:09:09Z\"");

    for expected in good_datetimes() {
        let s = format!("foo = {}", expected);
        good::<Datetime>(&s, expected);
    }
}

#[test]
fn test_spanned_vals() {
    fn assert_span_subspan(outer: (usize, usize), inner: (usize, usize)) {
        if outer == (0, 0) || inner == (0, 0) {
            // One of the spans is not available.
            // In the general case, the toml format doesn't allow valid spans
            // to be created for dotted tables as well as arrays
            // that use [[]] syntax
            return;
        }
        // Allow the inner span to start xor end at the same place
        // as the outer one
        assert!((inner.1 - inner.0) < (outer.1 - outer.0));
        assert!(outer.0 <= inner.0);
        assert!(inner.1 <= outer.1);
    }
    fn visit(val: &SpannedValue, s: &str) {
        let substr = &s[val.start()..val.end()];
        match val.get_ref() {
            K::Array(_) | K::Table(_) => {}
            K::String(c) => assert_eq!(format!("\"{}\"", c), substr),
            K::Integer(c) => assert_eq!(Ok(c.clone()), i64::from_str_radix(substr, 10)),
            K::Float(c) => assert_eq!(Ok(c.clone()), f64::from_str(substr)),
            K::Boolean(c) => assert_eq!(format!("{}", c), substr.to_lowercase()),
            K::Datetime(c) => {
                let dt = Datetime::from_str(substr).unwrap();
                assert_eq!(c.clone(), dt);
            }
        }
        match val.get_ref() {
            K::Array(arr) => {
                for v in arr.iter() {
                    assert_span_subspan(val.span(), v.span());
                    visit(v, s);
                }
            }
            K::Table(tbl) => {
                for (key, v) in tbl.iter() {
                    assert_eq!(&s[key.start()..key.end()], key.get_ref());
                    assert_span_subspan(val.span(), key.span());
                    assert_span_subspan(val.span(), v.span());
                    visit(v, s);
                }
            }
            K::String(_) | K::Integer(_) | K::Float(_) | K::Boolean(_) | K::Datetime(_) => {}
        }
    }

    const TEST_TOML: &str = r#"
    # comments are supported
    key_foo = "bazz"
    key_baz = false
    empty_arr = []
    filled_arr_ints = [1, 2, 3, 4, 5]
    key_nested_baz = { extremely_nested = "yess", other_val = true }
    even_more_nesting = { u = 22, f = { s = { v = 32 }, w = 77 }, v = 33 }
    [tbl]
    hello = "world"
    d = "hi"
    [[arr]]
    this_is = "something in the array"
    [[arr]]
    this_is = "something else in the array"
    "#;
    let foo: SpannedValue = toml::from_str(TEST_TOML).unwrap();

    // Ensure that indexing still works
    assert_eq!(foo.get_ref()["key_baz"].get_ref(), &K::Boolean(false));

    visit(&foo, TEST_TOML);
}
