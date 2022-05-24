extern crate toml;

use std::str::FromStr;
use toml::value::{Date, Datetime, Offset, Time};

macro_rules! bad {
    ($toml:expr, $msg:expr) => {
        match $toml.parse::<toml::Value>() {
            Ok(s) => panic!("parsed to: {:#?}", s),
            Err(e) => assert_eq!(e.to_string(), $msg),
        }
    };
}

#[test]
fn times() {
    fn dogood(s: &str, serialized: &str) {
        let to_parse = format!("foo = {}", s);
        let value = toml::Value::from_str(&to_parse).unwrap();
        assert_eq!(value["foo"].as_datetime().unwrap().to_string(), serialized);
    }
    fn good(s: &str) {
        dogood(s, s);
        dogood(&s.replace("T", " "), s);
        dogood(&s.replace("T", "t"), s);
        dogood(&s.replace("Z", "z"), s);
    }

    good("1997-09-09T09:09:09Z");
    dogood("1997-09-09 09:09:09Z", "1997-09-09T09:09:09Z");
    good("1997-09-09T09:09:09+09:09");
    good("1997-09-09T09:09:09-09:09");
    good("1997-09-09T09:09:09");
    good("1997-09-09");
    dogood("1997-09-09 ", "1997-09-09");
    dogood("1997-09-09 # comment", "1997-09-09");
    good("09:09:09");
    good("1997-09-09T09:09:09.09Z");
    good("1997-09-09T09:09:09.09+09:09");
    good("1997-09-09T09:09:09.09-09:09");
    good("1997-09-09T09:09:09.09");
    good("09:09:09.09");
    good("2003-02-28T12:34:56Z");
    good("2000-02-29T12:34:56Z");
    good("2004-02-29T12:34:56Z");
}

#[test]
fn bad_times() {
    bad!(
        "foo = 199-09-09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 199709-09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-9-09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-9",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-0909:09:09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.",
        "invalid date at line 1 column 7"
    );
    bad!(
        "foo = T",
        "invalid TOML value, did you mean to use a quoted string? at line 1 column 7"
    );
    bad!(
        "foo = T.",
        "invalid TOML value, did you mean to use a quoted string? at line 1 column 7"
    );
    bad!(
        "foo = TZ",
        "invalid TOML value, did you mean to use a quoted string? at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09+",
        "invalid date at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09+09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09+09:9",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09+0909",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09-",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09-09",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09-09:9",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T09:09:09.09-0909",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );

    bad!(
        "foo = 1997-00-09T09:09:09.09Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-00T09:09:09.09Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T30:09:09.09Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T12:69:09.09Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1997-09-09T12:09:69.09Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 2003-02-29T12:34:56Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
    bad!(
        "foo = 1900-02-29T12:34:56Z",
        "failed to parse datetime for key `foo` at line 1 column 7"
    );
}

#[test]
fn datetime_custom_tz() {
    assert_eq!(
        Datetime::from_str("2015-06-26T16:43:23.123+02:00").unwrap(),
        Datetime {
            date: Some(Date {
                year: 2015,
                month: 6,
                day: 26,
            }),
            time: Some(Time {
                hour: 16,
                minute: 43,
                second: 23,
                nanosecond: 123_000_000,
            }),
            offset: Some(Offset::Custom {
                hours: 2,
                minutes: 0,
            }),
        }
    );
}

#[test]
fn datetime_z() {
    assert_eq!(
        Datetime::from_str("2015-06-26T16:43:23Z").unwrap(),
        Datetime {
            date: Some(Date {
                year: 2015,
                month: 6,
                day: 26,
            }),
            time: Some(Time {
                hour: 16,
                minute: 43,
                second: 23,
                nanosecond: 0,
            }),
            offset: Some(Offset::Z),
        }
    );
}

#[test]
fn datetime_naive() {
    assert_eq!(
        Datetime::from_str("2015-06-26T16:43:23.001234").unwrap(),
        Datetime {
            date: Some(Date {
                year: 2015,
                month: 6,
                day: 26,
            }),
            time: Some(Time {
                hour: 16,
                minute: 43,
                second: 23,
                nanosecond: 1_234_000,
            }),
            offset: None,
        }
    );
}

#[test]
fn date() {
    assert_eq!(
        Datetime::from_str("2015-06-26").unwrap(),
        Datetime {
            date: Some(Date {
                year: 2015,
                month: 6,
                day: 26,
            }),
            time: None,
            offset: None,
        }
    );
}

#[test]
fn time() {
    assert_eq!(
        Datetime::from_str("16:43:23.1").unwrap(),
        Datetime {
            date: None,
            time: Some(Time {
                hour: 16,
                minute: 43,
                second: 23,
                nanosecond: 100_000_000,
            }),
            offset: None,
        }
    );
}
