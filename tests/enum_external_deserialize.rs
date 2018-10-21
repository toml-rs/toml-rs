#[macro_use]
extern crate serde_derive;
extern crate toml;

#[derive(Debug, Deserialize, PartialEq)]
enum TheEnum {
    Plain,
    Tuple(i64, bool),
    NewType(String),
    Struct { value: i64 },
}

#[derive(Debug, Deserialize, PartialEq)]
struct Val {
    val: TheEnum,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Multi {
    enums: Vec<TheEnum>,
}

mod enum_unit {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(TheEnum::Plain, toml::from_str("\"Plain\"").unwrap());
    }

    #[test]
    fn from_inline_table() {
        assert_eq!(TheEnum::Plain, toml::from_str("{ Plain = {} }").unwrap());
        assert_eq!(
            Val {
                val: TheEnum::Plain
            },
            toml::from_str("val = { Plain = {} }").unwrap()
        );
    }

    #[test]
    fn from_dotted_table() {
        assert_eq!(TheEnum::Plain, toml::from_str("[Plain]\n").unwrap());
    }
}

mod enum_tuple {
    use super::*;

    #[test]
    fn from_inline_table() {
        assert_eq!(
            TheEnum::Tuple(-123, true),
            toml::from_str("{ Tuple = { 0 = -123, 1 = true } }").unwrap()
        );
        assert_eq!(
            Val {
                val: TheEnum::Tuple(-123, true)
            },
            toml::from_str("val = { Tuple = { 0 = -123, 1 = true } }").unwrap()
        );
    }

    #[test]
    fn from_dotted_table() {
        assert_eq!(
            TheEnum::Tuple(-123, true),
            toml::from_str(
                r#"[Tuple]
                0 = -123
                1 = true
                "#
            )
            .unwrap()
        );
    }
}

mod enum_newtype {
    use super::*;

    #[test]
    fn from_inline_table() {
        assert_eq!(
            TheEnum::NewType("value".to_string()),
            toml::from_str(r#"{ NewType = "value" }"#).unwrap()
        );
        assert_eq!(
            Val {
                val: TheEnum::NewType("value".to_string()),
            },
            toml::from_str(r#"val = { NewType = "value" }"#).unwrap()
        );
    }
}

mod enum_struct {
    use super::*;

    #[test]
    fn from_inline_table() {
        assert_eq!(
            TheEnum::Struct { value: -123 },
            toml::from_str("{ Struct = { value = -123 } }").unwrap()
        );
        assert_eq!(
            Val {
                val: TheEnum::Struct { value: -123 }
            },
            toml::from_str("val = { Struct = { value = -123 } }").unwrap()
        );
    }

    #[test]
    fn from_dotted_table() {
        assert_eq!(
            TheEnum::Struct { value: -123 },
            toml::from_str(
                r#"[Struct]
                value = -123
                "#
            )
            .unwrap()
        );
    }
}

mod enum_array {
    use super::*;

    #[test]
    fn from_inline_tables() {
        let toml_str = r#"
            enums = [
                { Plain = {} },
                { Tuple = { 0 = -123, 1 = true } },
                { NewType = "value" },
                { Struct = { value = -123 } }
            ]"#;
        assert_eq!(
            Multi {
                enums: vec![
                    TheEnum::Plain,
                    TheEnum::Tuple(-123, true),
                    TheEnum::NewType("value".to_string()),
                    TheEnum::Struct { value: -123 },
                ]
            },
            toml::from_str(toml_str).unwrap()
        );
    }
}
