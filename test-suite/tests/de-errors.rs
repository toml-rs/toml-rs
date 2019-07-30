extern crate serde;
extern crate toml;

use serde::{de, Deserialize};
use std::fmt;

macro_rules! bad {
    ($toml:expr, $ty:ty, $msg:expr) => {
        match toml::from_str::<$ty>($toml) {
            Ok(s) => panic!("parsed to: {:#?}", s),
            Err(e) => assert_eq!(e.to_string(), $msg),
        }
    };
}

#[derive(Debug, Deserialize, PartialEq)]
struct Parent<T> {
    p_a: T,
    p_b: Vec<Child<T>>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Child<T> {
    c_a: T,
    c_b: T,
}

#[derive(Debug, PartialEq)]
enum CasedString {
    Lowercase(String),
    Uppercase(String),
}

impl<'de> de::Deserialize<'de> for CasedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct CasedStringVisitor;

        impl<'de> de::Visitor<'de> for CasedStringVisitor {
            type Value = CasedString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if s.is_empty() {
                    Err(de::Error::invalid_length(0, &"a non-empty string"))
                } else if s.chars().all(|x| x.is_ascii_lowercase()) {
                    Ok(CasedString::Lowercase(s.to_string()))
                } else if s.chars().all(|x| x.is_ascii_uppercase()) {
                    Ok(CasedString::Uppercase(s.to_string()))
                } else {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(s),
                        &"all lowercase or all uppercase",
                    ))
                }
            }
        }

        deserializer.deserialize_any(CasedStringVisitor)
    }
}

#[test]
fn custom_errors() {
    toml::from_str::<Parent<CasedString>>(
        "
            p_a = 'a'
            p_b = [{c_a = 'a', c_b = 'c'}]
        ",
    )
    .unwrap();

    // Custom error at p_b value.
    bad!(
        "
            p_a = ''
        ",
        Parent<CasedString>,
        "invalid length 0, expected a non-empty string for key `p_a`"
    );

    // Missing field in table.
    bad!(
        "
            p_a = 'a'
        ",
        Parent<CasedString>,
        "missing field `p_b`"
    );

    // Invalid type in p_b.
    bad!(
        "
            p_a = 'a'
            p_b = 1
        ",
        Parent<CasedString>,
        "invalid type: integer `1`, expected a sequence for key `p_b`"
    );

    // Sub-table in Vec is missing a field.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a'}
            ]
        ",
        Parent<CasedString>,
        "missing field `c_b` for key `p_b`"
    );

    // Sub-table in Vec has a field with a bad value.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a', c_b = '*'}
            ]
        ",
        Parent<CasedString>,
        "invalid value: string \"*\", expected all lowercase or all uppercase for key `p_b`"
    );

    // Sub-table in Vec is missing a field.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a', c_b = 'b'},
                {c_a = 'aa'}
            ]
        ",
        Parent<CasedString>,
        "missing field `c_b` for key `p_b`"
    );

    // Sub-table in the middle of a Vec is missing a field.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a', c_b = 'b'},
                {c_a = 'aa'},
                {c_a = 'aaa', c_b = 'bbb'},
            ]
        ",
        Parent<CasedString>,
        "missing field `c_b` for key `p_b`"
    );

    // Sub-table in the middle of a Vec has a field with a bad value.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a', c_b = 'b'},
                {c_a = 'aa', c_b = 1},
                {c_a = 'aaa', c_b = 'bbb'},
            ]
        ",
        Parent<CasedString>,
        "invalid type: integer `1`, expected a string for key `p_b`"
    );

    // Sub-table in the middle of a Vec has an extra field.
    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = 'a', c_b = 'b'},
                {c_a = 'aa', c_b = 'bb', c_d = 'd'},
                {c_a = 'aaa', c_b = 'bbb'},
            ]
        ",
        Parent<CasedString>,
        "unknown field `c_d`, expected `c_a` or `c_b` for key `p_b`" // FIX ME
    );

    // Sub-table in the middle of a Vec is missing a field.
    bad!(
        "
            p_a = 'a'
            [[p_b]]
            c_a = 'a'
            c_b = 'b'
            [[p_b]]
            c_a = 'aa'
            [[p_b]]
            c_a = 'aaa'
            c_b = 'bbb'
        ",
        Parent<CasedString>,
        "missing field `c_b` for key `p_b`"
    );

    // Sub-table in the middle of a Vec has a field with a bad value.
    bad!(
        "
            p_a = 'a'
            [[p_b]]
            c_a = 'a'
            c_b = 'b'
            [[p_b]]
            c_a = 'aa'
            c_b = '*'
            [[p_b]]
            c_a = 'aaa'
            c_b = 'bbb'
        ",
        Parent<CasedString>,
        "invalid value: string \"*\", expected all lowercase or all uppercase for key `p_b.c_b`"
    );

    // Sub-table in the middle of a Vec has an extra field.
    bad!(
        "
            p_a = 'a'
            [[p_b]]
            c_a = 'a'
            c_b = 'b'
            [[p_b]]
            c_a = 'aa'
            c_d = 'dd' # unknown field
            [[p_b]]
            c_a = 'aaa'
            c_b = 'bbb'
        ",
        Parent<CasedString>,
        "unknown field `c_d`, expected `c_a` or `c_b` for key `p_b`"
    );
}

#[test]
fn serde_derive_deserialize_errors() {
    bad!(
        "
            p_a = ''
        ",
        Parent<String>,
        "missing field `p_b`"
    );

    bad!(
        "
            p_a = ''
            p_b = [
                {c_a = ''}
            ]
        ",
        Parent<String>,
        "missing field `c_b` for key `p_b`"
    );

    bad!(
        "
            p_a = ''
            p_b = [
                {c_a = '', c_b = 1}
            ]
        ",
        Parent<String>,
        "invalid type: integer `1`, expected a string for key `p_b`"
    );

    bad!(
        "
            p_a = ''
            p_b = [
                {c_a = '', c_b = '', c_d = ''},
            ]
        ",
        Parent<String>,
        "unknown field `c_d`, expected `c_a` or `c_b` for key `p_b`" // FIX ME
    );

    bad!(
        "
            p_a = 'a'
            p_b = [
                {c_a = '', c_b = 1, c_d = ''},
            ]
        ",
        Parent<String>,
        "invalid type: integer `1`, expected a string for key `p_b`"
    );
}
