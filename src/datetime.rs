//! Date and time types.

use serde::ser::{Serialize, Serializer, SerializeStruct};
use serde::de::{Deserialize, Deserializer, self};
use std::{error, fmt};
use std::str::{Chars, FromStr};

pub(crate) const NAME:  &'static str = "$toml::private::Datetime";
pub(crate) const FIELD: &'static str = "$toml::private::Datetime";

//TODO(quadrupleslap): Consider renaming all `Datetime` to `DateTime`.
//TODO(quadrupleslap): This makes the output subsecond a multiple of three digits long.
//TODO(quadrupleslap): Better error messages that no one will use.

#[derive(Clone, Debug, PartialEq, Eq)]
/// A parsed TOML datetime value.
///
/// This structure is intended to represent the datetime primitive type that can
/// be encoded into TOML documents. This type is a parsed version that contains
/// all metadata internally.
///
/// Note that if you're using `Deserialize` to deserialize a TOML document, you
/// can use this as a placeholder for where you're expecting a datetime to be
/// specified.
///
/// Also note though that while this type implements `Serialize` and
/// `Deserialize` it's only recommended to use this type with the TOML format,
/// otherwise encoded in other formats it may look a little odd.
pub enum Datetime {
    /// An date, a time and a timezone.
    OffsetDatetime(chrono::DateTime<chrono::FixedOffset>),
    /// A date and a time.
    LocalDatetime(chrono::NaiveDateTime),
    /// A date.
    LocalDate(chrono::NaiveDate),
    /// A time.
    LocalTime(chrono::NaiveTime),
}

impl FromStr for Datetime {
    type Err = DatetimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Let Y, M, D, H, M and S all represent a single digit.
        //
        // LocalDate      : YYYY-MM-DD
        // LocalTime      : HH:MM:SS(\.S+)?
        // LocalDatetime  : { LocalDate }[Tt ]{ LocalTime }
        // OffsetDatetime : { OffsetDatetime }(Z|[+-]HH:MM)

        let chars = &mut s.chars();

        if !s.contains('-') {
            let time = parse_time(chars)?;
            expect_end(chars)?;
            return Ok(Datetime::LocalTime(time));
        }

        let date = parse_date(chars)?;

        match chars.next() {
            None => return Ok(Datetime::LocalDate(date)),
            Some('T') | Some('t') | Some(' ') => (),
            Some(_) => return Err(DatetimeParseError::new()),
        }

        let time = parse_time(chars)?;
        let datetime = date.and_time(time);

        let offset_branch = &mut chars.clone();
        Ok(if let Ok(offset) = parse_offset(offset_branch) {
            expect_end(offset_branch)?;
            Datetime::OffsetDatetime(chrono::DateTime::from_utc(datetime - offset, offset))
        } else {
            expect_end(chars)?;
            Datetime::LocalDatetime(datetime)
        })
    }
}

/// Z|[+-]HH:MM
fn parse_offset(chars: &mut Chars) -> Result<chrono::FixedOffset, DatetimeParseError> {
    if accept(chars, 'Z') { return Ok(chrono::FixedOffset::east(0)) }
    let sign = if accept(chars, '+') { 1 } else { expect(chars, '-')?; -1 };
    let h = digits(chars, 2)?;
    expect(chars, ':')?;
    let m = digits(chars, 2)?;
    match chrono::FixedOffset::east_opt(sign*60*(60*h + m)) {
        Some(offset) => Ok(offset),
        None => Err(DatetimeParseError::new()),
    }
}

/// HH:MM:SS(\.S+)?
fn parse_time(chars: &mut Chars) -> Result<chrono::NaiveTime, DatetimeParseError> {
    let h = digits(chars, 2)?;
    expect(chars, ':')?;
    let m = digits(chars, 2)?;
    expect(chars, ':')?;
    let s = digits(chars, 2)?;
    let n = if accept(chars, '.') { fract(chars, 9)? } else { 0 };
    match chrono::NaiveTime::from_hms_nano_opt(h as _, m as _, s as _, n as _) {
        Some(time) => Ok(time),
        None => Err(DatetimeParseError::new()),
    }
}

/// YYYY-MM-DD
fn parse_date(chars: &mut Chars) -> Result<chrono::NaiveDate, DatetimeParseError> {
    let y = digits(chars, 4)?;
    expect(chars, '-')?;
    let m = digits(chars, 2)?;
    expect(chars, '-')?;
    let d = digits(chars, 2)?;
    match chrono::NaiveDate::from_ymd_opt(y as _, m as _, d as _) {
        Some(date) => Ok(date),
        None => Err(DatetimeParseError::new()),
    }
}

fn expect(chars: &mut Chars, c: char) -> Result<(), DatetimeParseError> {
    if chars.next() == Some(c) {
        Ok(())
    } else {
        Err(DatetimeParseError::new())
    }
}

fn expect_end(chars: &mut Chars) -> Result<(), DatetimeParseError> {
    if chars.next() == None {
        Ok(())
    } else {
        Err(DatetimeParseError::new())
    }
}

fn accept(chars: &mut Chars, c: char) -> bool {
    if chars.clone().next() == Some(c) {
        let _ = chars.next();
        true
    } else {
        false
    }
}

/// [0-9]+
fn fract(chars: &mut Chars, n: usize) -> Result<i32, DatetimeParseError> {
    let mut x = digit(chars)? as i32;
    let mut i = 1;
    while let Ok(d) = digit(&mut chars.clone()) {
        let _ = chars.next();
        if i < n { x = 10*x + d as i32 }
        i += 1;
    }
    while i < n {
        x *= 10;
        i += 1;
    }
    Ok(x)
}

/// [0-9]
fn digit(chars: &mut Chars) -> Result<u8, DatetimeParseError> {
    match chars.next() {
        Some(c) if '0' <= c && c <= '9' => Ok(c as u8 - b'0'),
        _ => Err(DatetimeParseError::new()),
    }
}

/// [0-9]{n}
fn digits(chars: &mut Chars, n: usize) -> Result<i32, DatetimeParseError> {
    let mut x = 0i32;
    for _ in 0..n {
        x = 10*x + digit(chars)? as i32;
    }
    Ok(x)
}

impl Serialize for Datetime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct(NAME, 1)?;
        match self {
            Datetime::OffsetDatetime(x) => s.serialize_field(FIELD, x)?,
            Datetime::LocalDatetime(x) => s.serialize_field(FIELD, x)?,
            Datetime::LocalDate(x) => s.serialize_field(FIELD, x)?,
            Datetime::LocalTime(x) => s.serialize_field(FIELD, x)?,
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for Datetime {
    fn deserialize<D>(deserializer: D) -> Result<Datetime, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DatetimeKey;
        struct DatetimeVisitor;

        impl<'de> de::Deserialize<'de> for DatetimeKey {
            fn deserialize<D>(deserializer: D) -> Result<DatetimeKey, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a valid datetime field")
                    }

                    fn visit_str<E>(self, s: &str) -> Result<(), E>
                    where
                        E: de::Error,
                    {
                        if s == FIELD {
                            Ok(())
                        } else {
                            Err(de::Error::custom("expected field with custom name"))
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)?;
                Ok(DatetimeKey)
            }
        }

        impl<'de> de::Visitor<'de> for DatetimeVisitor {
            type Value = Datetime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a TOML datetime")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Datetime, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let value = visitor.next_key::<DatetimeKey>()?;
                if value.is_none() {
                    return Err(de::Error::custom("datetime key not found"));
                }
                let v: DatetimeFromString = visitor.next_value()?;
                Ok(v.value)
            }
        }

        static FIELDS: [&'static str; 1] = [FIELD];
        deserializer.deserialize_struct(NAME, &FIELDS, DatetimeVisitor)
    }
}

impl fmt::Display for Datetime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Datetime::OffsetDatetime(x) => {
                let local = x.naive_local();
                let mut offset = x.offset().local_minus_utc() / 60;

                write!(f, "{}T{}", local.date(), local.time())?;

                if offset == 0 {
                    return write!(f, "Z");
                }

                if offset > 0 {
                    write!(f, "+")?;
                } else {
                    write!(f, "-")?;
                    offset = -offset;
                }

                write!(f, "{:02}:{:02}", offset / 60, offset % 60)
            },
            Datetime::LocalDatetime(x) => write!(f, "{}T{}", x.date(), x.time()),
            Datetime::LocalDate(x) => x.fmt(f),
            Datetime::LocalTime(x) => x.fmt(f),
        }
    }
}

pub struct DatetimeFromString {
    pub value: Datetime,
}

impl<'de> de::Deserialize<'de> for DatetimeFromString {
    fn deserialize<D>(deserializer: D) -> Result<DatetimeFromString, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DatetimeFromString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string containing a datetime")
            }

            fn visit_str<E>(self, s: &str) -> Result<DatetimeFromString, E>
            where
                E: de::Error,
            {
                match s.parse() {
                    Ok(date) => Ok(DatetimeFromString { value: date }),
                    Err(e) => Err(de::Error::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Error returned from parsing a `Datetime` in the `FromStr` implementation.
#[derive(Debug, Clone)]
pub struct DatetimeParseError {
    _priv: (),
}

impl DatetimeParseError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl fmt::Display for DatetimeParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "failed to parse datetime".fmt(f)
    }
}

impl error::Error for DatetimeParseError {}
