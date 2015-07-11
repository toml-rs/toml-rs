extern crate toml;

use toml::{Parser};

fn run(toml: &str) {
    let mut p = Parser::new(toml);
    let table = p.parse_doc();
    assert!(p.errors.len() == 0, "had_errors: {:?}",
            p.errors.iter().map(|e| {
                (e.desc.clone(), &toml[e.lo - 5..e.hi + 5])
            }).collect::<Vec<(String, &str)>>());
    assert!(table.is_some());
    let mut str_buf = String::new();
    table.unwrap().serialize(&mut str_buf);
    assert!(toml == &*str_buf,
            "expected:\n{}\nactual:\n{}\n",
            toml,
            str_buf);
}

macro_rules! test( ($name:ident, $toml:expr) => (
    #[test]
    fn $name() { run($toml); }
) );

test!(array_empty,
       include_str!("valid/array-empty.toml"));
test!(array_nospaces,
       include_str!("valid/array-nospaces.toml"));
test!(arrays_hetergeneous,
       include_str!("valid/arrays-hetergeneous.toml"));
test!(arrays,
       include_str!("valid/arrays.toml"));
test!(arrays_nested,
       include_str!("valid/arrays-nested.toml"));
test!(empty,
       include_str!("valid/empty.toml"));
test!(bool,
       include_str!("valid/bool.toml"));
test!(datetime,
       include_str!("valid/datetime.toml"));
test!(example,
       include_str!("valid/example.toml"));
test!(float,
       include_str!("valid/float.toml"));
test!(implicit_and_explicit_after,
       include_str!("valid/implicit-and-explicit-after.toml"));
test!(implicit_and_explicit_before,
       include_str!("valid/implicit-and-explicit-before.toml"));
test!(implicit_groups,
       include_str!("valid/implicit-groups.toml"));
test!(integer,
       include_str!("valid/integer.toml"));
test!(key_equals_nospace,
       include_str!("valid/key-equals-nospace.toml"));
test!(key_special_chars,
       include_str!("valid/key-special-chars.toml"));
test!(key_with_pound,
       include_str!("valid/key-with-pound.toml"));
test!(long_float,
       include_str!("valid/long-float.toml"));
test!(long_integer,
       include_str!("valid/long-integer.toml"));
test!(string_empty,
       include_str!("valid/string-empty.toml"));
test!(string_escapes,
       include_str!("valid/string-escapes.toml"));
test!(string_simple,
       include_str!("valid/string-simple.toml"));
test!(string_with_pound,
       include_str!("valid/string-with-pound.toml"));
test!(table_array_implicit,
       include_str!("valid/table-array-implicit.toml"));
test!(table_array_many,
       include_str!("valid/table-array-many.toml"));
test!(table_array_nest,
       include_str!("valid/table-array-nest.toml"));
test!(table_array_one,
       include_str!("valid/table-array-one.toml"));
test!(table_empty,
       include_str!("valid/table-empty.toml"));
test!(table_sub_empty,
       include_str!("valid/table-sub-empty.toml"));
test!(table_whitespace,
       include_str!("valid/table-whitespace.toml"));
test!(table_with_pound,
       include_str!("valid/table-with-pound.toml"));
test!(unicode_escape,
       include_str!("valid/unicode-escape.toml"));
test!(unicode_literal,
       include_str!("valid/unicode-literal.toml"));
test!(hard_example,
       include_str!("valid/hard_example.toml"));
test!(example2,
       include_str!("valid/example2.toml"));
test!(example3,
       include_str!("valid/example-v0.3.0.toml"));
test!(example4,
       include_str!("valid/example-v0.4.0.toml"));
