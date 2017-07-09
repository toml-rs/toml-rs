extern crate toml;
extern crate serde;

use serde::ser::Serialize;

const EXAMPLE: &str = "\
[example]
text = '''
this is the first line
this is the second line
'''
";

#[test]
fn test_pretty() {
    let value: toml::Value = toml::from_str(EXAMPLE).unwrap();
    let mut result = String::with_capacity(128);
    value.serialize(&mut toml::Serializer::pretty(&mut result)).unwrap();
    assert_eq!(EXAMPLE, &result);
}
