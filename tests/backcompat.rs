extern crate toml;
extern crate serde;

use serde::de::Deserialize;

#[test]
fn main() {
    assert!("[a] foo = 1".parse::<toml::Value>().is_err());

    let mut d = toml::de::Deserializer::new("[a] foo = 1");
    d.set_require_newline_after_table(false);
    let value = toml::Value::deserialize(&mut d).unwrap();
    assert_eq!(value["a"]["foo"].as_integer(), Some(1));
}
