extern crate toml;
use toml::Value;

#[test]
fn bad() {
    fn bad(s: &str) {
        assert!(s.parse::<toml::Value>().is_err());
    }

    bad("a = 01");
    bad("a = 1__1");
    bad("a = 1_");
    bad("''");
    bad("a = nan");
    bad("a = -inf");
    bad("a = inf");
    bad("a = 9e99999");
}

#[test]
fn inserting_value() {

    let insert_error = "failed to insert value";
    let cast_error = "failed to cast value";

    let mut some_value: Value = toml::from_str("a=1").expect("failed to create Value");
    some_value
        .insert("b", Value::Integer(2))
        .expect(insert_error);
    some_value
        .insert("c", Value::Integer(3))
        .expect(insert_error);

    assert_eq!(some_value["a"].as_integer().expect(cast_error), 1);
    assert_eq!(some_value["b"].as_integer().expect(cast_error), 2);
    assert_eq!(some_value["c"].as_integer().expect(cast_error), 3);
}
