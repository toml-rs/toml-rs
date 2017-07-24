extern crate toml;
extern crate serde;

use serde::ser::Serialize;

const NO_PRETTY: &'static str = "\
[example]
array = [\"item 1\", \"item 2\"]
empty = []
oneline = \"this has no newlines.\"
text = \"\\nthis is the first line\\nthis is the second line\\n\"
";

#[test]
fn no_pretty() {
    let toml = NO_PRETTY;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    value.serialize(&mut toml::Serializer::new(&mut result)).unwrap();
    println!("EXPECTED:\n{}", toml);
    println!("\nRESULT:\n{}", result);
    assert_eq!(toml, &result);
}

#[test]
fn disable_pretty() {
    let toml = NO_PRETTY;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    {
        let mut serializer = toml::Serializer::pretty(&mut result);
        serializer.pretty_string(false);
        serializer.pretty_array(false);
        value.serialize(&mut serializer).unwrap();
    }
    println!("EXPECTED:\n{}", toml);
    println!("\nRESULT:\n{}", result);
    assert_eq!(toml, &result);
}

const PRETTY_STD: &'static str = "\
[example]
array = [
    \"item 1\",
    \"item 2\",
]
empty = []
oneline = \"this has no newlines.\"
text = '''
this is the first line
this is the second line
'''
";

#[test]
fn pretty_std() {
    let toml = PRETTY_STD;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    value.serialize(&mut toml::Serializer::pretty(&mut result)).unwrap();
    println!("EXPECTED:\n{}", toml);
    println!("\nRESULT:\n{}", result);
    assert_eq!(toml, &result);
}


const PRETTY_INDENT_2: &'static str = "\
[example]
array = [
  \"item 1\",
  \"item 2\",
]
empty = []
oneline = \"this has no newlines.\"
text = '''
this is the first line
this is the second line
'''
";

#[test]
fn pretty_indent_2() {
    let toml = PRETTY_INDENT_2;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    {
        let mut serializer = toml::Serializer::pretty(&mut result);
        serializer.pretty_array_indent(2);
        value.serialize(&mut serializer).unwrap();
    }
    assert_eq!(toml, &result);
}

const PRETTY_INDENT_2_OTHER: &'static str = "\
[example]
array = [
  \"item 1\",
  \"item 2\",
]
empty = []
oneline = \"this has no newlines.\"
text = \"\\nthis is the first line\\nthis is the second line\\n\"
";


#[test]
/// Test pretty indent when gotten the other way
fn pretty_indent_2_other() {
    let toml = PRETTY_INDENT_2_OTHER;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    {
        let mut serializer = toml::Serializer::new(&mut result);
        serializer.pretty_array_indent(2);
        value.serialize(&mut serializer).unwrap();
    }
    assert_eq!(toml, &result);
}


const PRETTY_ARRAY_NO_COMMA: &'static str = "\
[example]
array = [
    \"item 1\",
    \"item 2\"
]
empty = []
oneline = \"this has no newlines.\"
text = \"\\nthis is the first line\\nthis is the second line\\n\"
";
#[test]
/// Test pretty indent when gotten the other way
fn pretty_indent_array_no_comma() {
    let toml = PRETTY_ARRAY_NO_COMMA;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    {
        let mut serializer = toml::Serializer::new(&mut result);
        serializer.pretty_array_trailing_comma(false);
        value.serialize(&mut serializer).unwrap();
    }
    assert_eq!(toml, &result);
}


const PRETTY_NO_STRING: &'static str = "\
[example]
array = [
    \"item 1\",
    \"item 2\",
]
empty = []
oneline = \"this has no newlines.\"
text = \"\\nthis is the first line\\nthis is the second line\\n\"
";
#[test]
/// Test pretty indent when gotten the other way
fn pretty_no_string() {
    let toml = PRETTY_NO_STRING;
    let value: toml::Value = toml::from_str(toml).unwrap();
    let mut result = String::with_capacity(128);
    {
        let mut serializer = toml::Serializer::pretty(&mut result);
        serializer.pretty_string(false);
        value.serialize(&mut serializer).unwrap();
    }
    assert_eq!(toml, &result);
}
