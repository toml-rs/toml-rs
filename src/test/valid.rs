extern crate serialize;

use std::num::strconv;
use std::collections::TreeMap;
use self::serialize::json;

use {Parser, Value, Table, String, Integer, Float, Boolean, Datetime, Array};

fn to_json(toml: Value) -> json::Json {
    fn doit(s: &str, json: json::Json) -> json::Json {
        let mut map = TreeMap::new();
        map.insert("type".to_string(), json::String(s.to_string()));
        map.insert("value".to_string(), json);
        json::Object(map)
    }
    match toml {
        String(s) => doit("string", json::String(s)),
        Integer(i) => doit("integer", json::String(i.to_string())),
        Float(f) => doit("float", json::String({
            let (bytes, _) =
                strconv::float_to_str_bytes_common(f, 10, true,
                                                   strconv::SignNeg,
                                                   strconv::DigMax(15),
                                                   strconv::ExpNone,
                                                   false);
            let s = String::from_utf8(bytes).unwrap();
            if s.as_slice().contains(".") {s} else {format!("{}.0", s)}
        })),
        Boolean(b) => doit("bool", json::String(b.to_string())),
        Datetime(s) => doit("datetime", json::String(s)),
        Array(arr) => {
            let is_table = match arr.as_slice().head() {
                Some(&Table(..)) => true,
                _ => false,
            };
            let json = json::List(arr.move_iter().map(to_json).collect());
            if is_table {json} else {doit("array", json)}
        }
        Table(table) => json::Object(table.move_iter().map(|(k, v)| {
            (k, to_json(v))
        }).collect()),
    }
}

fn run(toml: &str, json: &str) {
    let mut p = Parser::new(toml);
    let table = p.parse();
    assert!(p.errors.len() == 0, "had_errors: {}",
            p.errors.iter().map(|e| {
                (e.desc.clone(), toml.slice(e.lo - 5, e.hi + 5))
            }).collect::<Vec<(String, &str)>>());
    assert!(table.is_some());
    let table = table.unwrap();

    let json = json::from_str(json).unwrap();
    let toml_json = to_json(Table(table));
    assert!(json == toml_json,
            "expected\n{}\ngot\n{}\n",
            json.to_pretty_str(),
            toml_json.to_pretty_str());
}

macro_rules! test( ($name:ident, $toml:expr, $json:expr) => (
    #[test]
    fn $name() { run($toml, $json); }
) )

test!(array_empty,
       include_str!("valid/array-empty.toml"),
       include_str!("valid/array-empty.json"))
test!(array_nospaces,
       include_str!("valid/array-nospaces.toml"),
       include_str!("valid/array-nospaces.json"))
test!(arrays_hetergeneous,
       include_str!("valid/arrays-hetergeneous.toml"),
       include_str!("valid/arrays-hetergeneous.json"))
test!(arrays,
       include_str!("valid/arrays.toml"),
       include_str!("valid/arrays.json"))
test!(arrays_nested,
       include_str!("valid/arrays-nested.toml"),
       include_str!("valid/arrays-nested.json"))
test!(empty,
       include_str!("valid/empty.toml"),
       include_str!("valid/empty.json"))
test!(bool,
       include_str!("valid/bool.toml"),
       include_str!("valid/bool.json"))
test!(datetime,
       include_str!("valid/datetime.toml"),
       include_str!("valid/datetime.json"))
test!(example,
       include_str!("valid/example.toml"),
       include_str!("valid/example.json"))
test!(float,
       include_str!("valid/float.toml"),
       include_str!("valid/float.json"))
test!(implicit_and_explicit_after,
       include_str!("valid/implicit-and-explicit-after.toml"),
       include_str!("valid/implicit-and-explicit-after.json"))
test!(implicit_and_explicit_before,
       include_str!("valid/implicit-and-explicit-before.toml"),
       include_str!("valid/implicit-and-explicit-before.json"))
test!(implicit_groups,
       include_str!("valid/implicit-groups.toml"),
       include_str!("valid/implicit-groups.json"))
test!(integer,
       include_str!("valid/integer.toml"),
       include_str!("valid/integer.json"))
test!(key_equals_nospace,
       include_str!("valid/key-equals-nospace.toml"),
       include_str!("valid/key-equals-nospace.json"))
test!(key_special_chars,
       include_str!("valid/key-special-chars.toml"),
       include_str!("valid/key-special-chars.json"))
test!(key_with_pound,
       include_str!("valid/key-with-pound.toml"),
       include_str!("valid/key-with-pound.json"))
test!(long_float,
       include_str!("valid/long-float.toml"),
       include_str!("valid/long-float.json"))
test!(long_integer,
       include_str!("valid/long-integer.toml"),
       include_str!("valid/long-integer.json"))
test!(string_escapes,
       include_str!("valid/string-escapes.toml"),
       include_str!("valid/string-escapes.json"))
test!(string_simple,
       include_str!("valid/string-simple.toml"),
       include_str!("valid/string-simple.json"))
test!(string_with_pound,
       include_str!("valid/string-with-pound.toml"),
       include_str!("valid/string-with-pound.json"))
test!(table_array_implicit,
       include_str!("valid/table-array-implicit.toml"),
       include_str!("valid/table-array-implicit.json"))
test!(table_array_many,
       include_str!("valid/table-array-many.toml"),
       include_str!("valid/table-array-many.json"))
test!(table_array_nest,
       include_str!("valid/table-array-nest.toml"),
       include_str!("valid/table-array-nest.json"))
test!(table_array_one,
       include_str!("valid/table-array-one.toml"),
       include_str!("valid/table-array-one.json"))
test!(table_empty,
       include_str!("valid/table-empty.toml"),
       include_str!("valid/table-empty.json"))
test!(table_sub_empty,
       include_str!("valid/table-sub-empty.toml"),
       include_str!("valid/table-sub-empty.json"))
test!(table_whitespace,
       include_str!("valid/table-whitespace.toml"),
       include_str!("valid/table-whitespace.json"))
test!(table_with_pound,
       include_str!("valid/table-with-pound.toml"),
       include_str!("valid/table-with-pound.json"))
test!(unicode_escape,
       include_str!("valid/unicode-escape.toml"),
       include_str!("valid/unicode-escape.json"))
test!(unicode_literal,
       include_str!("valid/unicode-literal.toml"),
       include_str!("valid/unicode-literal.json"))
