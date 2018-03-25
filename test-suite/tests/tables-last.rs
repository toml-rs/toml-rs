#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::collections::HashMap;

#[derive(Serialize)]
struct A {
    #[serde(serialize_with = "toml::ser::tables_last")]
    vals: HashMap<&'static str, Value>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Value {
    Map(HashMap<&'static str, &'static str>),
    Int(i32),
}

#[test]
fn always_works() {
    let mut a = A { vals: HashMap::new() };
    a.vals.insert("foo", Value::Int(0));

    let mut sub = HashMap::new();
    sub.insert("foo", "bar");
    a.vals.insert("bar", Value::Map(sub));

    toml::to_string(&a).unwrap();
}

#[derive(Serialize)]
struct B {
    property: bool,
}

#[derive(Serialize)]
struct C {
    b: B,
    // struct B will be serialized as a table, so this property must not appear
    // after it and must be serialized before b.
    property2: bool,
}

// Make sure that serializing with nested structs and property ordering always
// works.
#[test]
fn nested_struct_tables() {
    let c = C {
        property2: false,
        b: B { property: true },
    };
    toml::to_string(&c).unwrap();
}
