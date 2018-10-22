#[macro_use]
extern crate serde_derive;
extern crate toml;

#[derive(Debug, Serialize, PartialEq)]
enum TheEnum {
    Plain,
    Tuple(i64, bool),
    NewType(String),
    Struct { value: i64 },
}

#[derive(Debug, Serialize, PartialEq)]
struct Val {
    val: TheEnum,
}

#[derive(Debug, Serialize, PartialEq)]
struct Multi {
    enums: Vec<TheEnum>,
}

#[test]
fn enum_unit_serializes_to_string_when_standalone() {
    assert_eq!(r#""Plain""#, toml::to_string(&TheEnum::Plain).unwrap());
}

#[test]
fn enum_tuple_serializes_to_inline_table() {
    assert_eq!(
        r#"{ Tuple = { 0 = -123, 1 = true } }"#,
        toml::to_string(&TheEnum::Tuple(-123, true)).unwrap()
    );
}

#[test]
fn enum_newtype_serializes_to_inline_table() {
    assert_eq!(
        r#"{ NewType = "value" }"#,
        toml::to_string(&TheEnum::NewType("value".to_string())).unwrap()
    );
}

#[test]
fn enum_struct_serializes_to_inline_table() {
    assert_eq!(
        r#"{ Struct = { value = -123 } }"#,
        toml::to_string(&TheEnum::Struct { value: -123 }).unwrap()
    );
}

#[test]
fn array_of_variants_serializes_to_inline_tables() {
    let toml_str = r#"
        enums = [
            { Plain = {} },
            { Tuple = { 0 = -123, 1 = true } },
            { NewType = "value" },
            { Struct = { value = -123 } }
        ]"#;

    assert_eq!(
        toml_str,
        toml::to_string(&Multi {
            enums: vec![
                TheEnum::Plain,
                TheEnum::Tuple(-123, true),
                TheEnum::NewType("value".to_string()),
                TheEnum::Struct { value: -123 },
            ]
        })
        .unwrap(),
    );
}
