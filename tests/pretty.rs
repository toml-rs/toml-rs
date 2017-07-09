extern crate toml;
extern crate serde;

use serde::ser::Serialize;

const example: &str = "\
[example]
text = '''
this is the first line
this is the second line
'''
";

#[test]
fn test_pretty() {
    let value: toml::Value = toml::from_str(example).unwrap();
    let mut result = String::with_capacity(128);
    value.serialize(&mut toml::Serializer::pretty(&mut result)).unwrap();
    assert_eq!(example, &result);
}
