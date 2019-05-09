extern crate serde;
extern crate toml;

use serde::de::Deserialize;

#[test]
fn newlines_after_tables() {
    let s = "
        [a] foo = 1
        [[b]] foo = 1
    ";
    assert!(s.parse::<toml::Value>().is_err());

    let mut d = toml::de::Deserializer::new(s);
    d.set_require_newline_after_table(false);
    let value = toml::Value::deserialize(&mut d).unwrap();
    assert_eq!(value["a"]["foo"].as_integer(), Some(1));
    assert_eq!(value["b"][0]["foo"].as_integer(), Some(1));
}

#[test]
fn allow_duplicate_after_longer() {
    let s = "
        [dependencies.openssl-sys]
        version = 1

        [dependencies]
        libc = 1

        [dependencies]
        bitflags = 1
    ";
    assert!(s.parse::<toml::Value>().is_err());

    let mut d = toml::de::Deserializer::new(s);
    d.set_allow_duplicate_after_longer_table(true);
    let value = toml::Value::deserialize(&mut d).unwrap();
    assert_eq!(
        value["dependencies"]["openssl-sys"]["version"].as_integer(),
        Some(1)
    );
    assert_eq!(value["dependencies"]["libc"].as_integer(), Some(1));
    assert_eq!(value["dependencies"]["bitflags"].as_integer(), Some(1));
}
