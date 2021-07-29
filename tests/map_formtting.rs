// Test copied from https://github.com/alexcrichton/toml-rs/issues/219

extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;

use std::collections::HashMap;


#[derive(Serialize, Deserialize)]
struct Foo {
    bar: Bar,
    baz: HashMap<String, Bar>,
}

#[derive(Serialize, Deserialize)]
struct FooSwapped {
    baz: HashMap<String, Bar>,
    bar: Bar,
}

#[derive(Serialize, Deserialize)]
struct Bar {
    foo: u32
}

#[test]
fn tables_separated_by_empty_lines() {
    let expected = "[bar]
foo = 42

[baz.a]
foo = 0
";

    let mut baz = HashMap::new();
    baz.insert("a".into(), Bar { foo: 0 });

    let foo = Foo {
        bar: Bar {
            foo: 42,
        },
        baz: baz,
    };

    assert_eq!(toml::to_string_pretty(&foo).unwrap(), expected);
}

#[test]
fn tables_separated_by_empty_lines_swapped() {
    let expected = "[baz.a]
foo = 0

[bar]
foo = 42
";

    let mut baz = HashMap::new();
    baz.insert("a".into(), Bar { foo: 0 });

    let foo = FooSwapped {
        bar: Bar {
            foo: 42,
        },
        baz: baz,
    };

    assert_eq!(toml::to_string_pretty(&foo).unwrap(), expected);
}
