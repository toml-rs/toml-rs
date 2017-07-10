extern crate toml;
extern crate serde;

use serde::ser::Serialize;

const EXAMPLE: &'static str = "\
[example]
array = [
    \"item 1\",
    \"item 2\",
]
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
    println!("example:\n{}", EXAMPLE);
    println!("result:\n{}", result);
    assert_eq!(EXAMPLE, &result);
}
