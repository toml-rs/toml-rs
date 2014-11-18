use std::char;
use std::collections::{TreeMap, HashSet};
use std::error::Error;
use std::num::FromStrRadix;
use std::str;

use {Value, TomlTable};
use Value::{Array, Table, Float, Integer, Boolean, Datetime};

/// Parser for converting a string to a TOML `Value` instance.
///
/// This parser contains the string slice that is being parsed, and exports the
/// list of errors which have occurred during parsing.
pub struct Parser<'a> {
    input: &'a str,
    cur: str::CharOffsets<'a>,
    tables_defined: HashSet<String>,

    /// A list of all errors which have occurred during parsing.
    ///
    /// Not all parse errors are fatal, so this list is added to as much as
    /// possible without aborting parsing. If `None` is returned by `parse`, it
    /// is guaranteed that this list is not empty.
    pub errors: Vec<ParserError>,
}

/// A structure representing a parse error.
///
/// The data in this structure can be used to trace back to the original cause
/// of the error in order to provide diagnostics about parse errors.
#[deriving(Show)]
pub struct ParserError {
    /// The low byte at which this error is pointing at.
    pub lo: uint,
    /// One byte beyond the last character at which this error is pointing at.
    pub hi: uint,
    /// A human-readable description explaining what the error is.
    pub desc: String,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for a string.
    ///
    /// The parser can be executed by invoking the `parse` method.
    ///
    /// # Example
    ///
    /// ```
    /// let toml = r#"
    ///     [test]
    ///     foo = "bar"
    /// "#;
    ///
    /// let mut parser = toml::Parser::new(toml);
    /// match parser.parse() {
    ///     Some(value) => println!("found toml: {}", value),
    ///     None => {
    ///         println!("parse errors: {}", parser.errors);
    ///     }
    /// }
    /// ```
    pub fn new(s: &'a str) -> Parser<'a> {
        Parser {
            input: s,
            cur: s.char_indices(),
            errors: Vec::new(),
            tables_defined: HashSet::new(),
        }
    }

    /// Converts a byte offset from an error message to a (line, column) pair
    ///
    /// All indexes are 0-based.
    pub fn to_linecol(&self, offset: uint) -> (uint, uint) {
        let mut cur = 0;
        for (i, line) in self.input.lines().enumerate() {
            if cur + line.len() > offset {
                return (i, offset - cur)
            }
            cur += line.len() + 1;
        }
        return (self.input.lines().count(), 0)
    }

    fn next_pos(&self) -> uint {
        self.cur.clone().next().map(|p| p.val0()).unwrap_or(self.input.len())
    }

    // Returns true and consumes the next character if it matches `ch`,
    // otherwise do nothing and return false
    fn eat(&mut self, ch: char) -> bool {
        match self.cur.clone().next() {
            Some((_, c)) if c == ch => { self.cur.next(); true }
            Some(_) | None => false,
        }
    }

    fn expect(&mut self, ch: char) -> bool {
        if self.eat(ch) { return true }
        let mut it = self.cur.clone();
        let lo = it.next().map(|p| p.val0()).unwrap_or(self.input.len());
        let hi = it.next().map(|p| p.val0()).unwrap_or(self.input.len());
        self.errors.push(ParserError {
            lo: lo,
            hi: hi,
            desc: match self.cur.clone().next() {
                Some((_, c)) => format!("expected `{}`, but found `{}`", ch, c),
                None => format!("expected `{}`, but found eof", ch)
            }
        });
        false
    }

    // Consumes whitespace ('\t' and ' ') until another character (or EOF) is reached
    fn ws(&mut self) {
        loop {
            match self.cur.clone().next() {
                Some((_, '\t')) |
                Some((_, ' ')) => { self.cur.next(); }
                _ => break,
            }
        }
    }

    // Consumes the rest of the line after a comment character
    fn comment(&mut self) {
        match self.cur.clone().next() {
            Some((_, '#')) => {}
            _ => return,
        }
        for (_, ch) in self.cur {
            if ch == '\n' { break }
        }
    }

    /// Executes the parser, parsing the string contained within.
    ///
    /// This function will return the `TomlTable` instance if parsing is
    /// successful, or it will return `None` if any parse error or invalid TOML
    /// error occurs.
    ///
    /// If an error occurs, the `errors` field of this parser can be consulted
    /// to determine the cause of the parse failure.
    pub fn parse(&mut self) -> Option<TomlTable> {
        let mut ret = TreeMap::new();
        loop {
            self.ws();
            match self.cur.clone().next() {
                Some((_, '#')) => { self.comment(); }
                Some((_, '\n')) |
                Some((_, '\r')) => { self.cur.next(); }
                Some((start, '[')) => {
                    self.cur.next();
                    let array = self.eat('[');

                    // Parse the name of the section
                    let mut section = String::new();
                    for (pos, ch) in self.cur {
                        if ch == ']' { break }
                        if ch == '[' {
                            self.errors.push(ParserError {
                                lo: pos,
                                hi: pos + 1,
                                desc: format!("section names cannot contain \
                                               a `[` character"),
                            });
                            continue
                        }
                        section.push(ch);
                    }

                    if section.len() == 0 {
                        self.errors.push(ParserError {
                            lo: start,
                            hi: start + if array {3} else {1},
                            desc: format!("section name must not be empty"),
                        });
                        continue
                    } else if array && !self.expect(']') {
                        return None
                    }

                    // Build the section table
                    let mut table = TreeMap::new();
                    if !self.values(&mut table) { return None }
                    if array {
                        self.insert_array(&mut ret, section, Table(table), start)
                    } else {
                        self.insert_table(&mut ret, section, table, start)
                    }
                }
                Some(_) => {
                    if !self.values(&mut ret) { return None }
                }
                None if self.errors.len() == 0 => return Some(ret),
                None => return None,
            }
        }
    }

    // Parses the values into the given TomlTable. Returns true in case of success
    // and false in case of error.
    fn values(&mut self, into: &mut TomlTable) -> bool {
        loop {
            self.ws();
            match self.cur.clone().next() {
                Some((_, '#')) => self.comment(),
                Some((_, '\n')) |
                Some((_, '\r')) => { self.cur.next(); }
                Some((_, '[')) => break,
                Some((start, _)) => {
                    let mut key = String::new();
                    let mut found_eq = false;
                    for (pos, ch) in self.cur {
                        match ch {
                            ' ' | '\t' => break,
                            '=' => { found_eq = true; break }
                            '\n' => {
                                self.errors.push(ParserError {
                                    lo: start,
                                    hi: pos + 1,
                                    desc: format!("keys cannot be defined \
                                                   across lines"),
                                })
                            }
                            c => key.push(c),
                        }
                    }
                    if !found_eq {
                        self.ws();
                        if !self.expect('=') { return false }
                    }

                    let value = match self.value() {
                        Some(value) => value,
                        None => return false,
                    };
                    self.insert(into, key, value, start);
                    self.ws();
                    self.comment();
                    self.eat('\r');
                    self.eat('\n');
                }
                None => break,
            }
        }
        return true
    }

    // Parses a value
    fn value(&mut self) -> Option<Value> {
        self.ws();
        match self.cur.clone().next() {
            Some((pos, '"')) => self.string(pos),
            Some((pos, '\'')) => self.literal_string(pos),
            Some((pos, 't')) |
            Some((pos, 'f')) => self.boolean(pos),
            Some((pos, '[')) => self.array(pos),
            Some((pos, '-')) => self.number_or_datetime(pos),
            Some((pos, ch)) if ch.is_digit() => self.number_or_datetime(pos),
            _ => {
                let mut it = self.cur.clone();
                let lo = it.next().map(|p| p.val0()).unwrap_or(self.input.len());
                let hi = it.next().map(|p| p.val0()).unwrap_or(self.input.len());
                self.errors.push(ParserError {
                    lo: lo,
                    hi: hi,
                    desc: format!("expected a value"),
                });
                return None
            }
        }
    }

    // Parses a single or multi-line string
    fn string(&mut self, start: uint) -> Option<Value> {
        if !self.expect('"') { return None }
        let mut multiline = false;
        let mut ret = String::new();

        // detect multiline literals, but be careful about empty ""
        // strings
        if self.eat('"') {
            if self.eat('"') {
                multiline = true;
                self.eat('\n');
            } else {
                // empty
                return Some(Value::String(ret))
            }
        }

        loop {
            match self.cur.next() {
                Some((_, '"')) => {
                    if multiline {
                        if !self.eat('"') { ret.push_str("\""); continue }
                        if !self.eat('"') { ret.push_str("\"\""); continue }
                    }
                    break
                }
                Some((pos, '\\')) => {
                    match escape(self, pos, multiline) {
                        Some(c) => ret.push(c),
                        None => {}
                    }
                }
                Some((_, '\n')) |
                Some((_, '\r')) if multiline => ret.push('\n'),
                Some((pos, ch)) if ch < '\u001f' => {
                    let mut escaped = String::new();
                    ch.escape_default(|c| escaped.push(c));
                    self.errors.push(ParserError {
                        lo: pos,
                        hi: pos + 1,
                        desc: format!("control character `{}` must be escaped",
                                      escaped)
                    });
                }
                Some((_, ch)) => ret.push(ch),
                None => {
                    self.errors.push(ParserError {
                        lo: start,
                        hi: self.input.len(),
                        desc: format!("unterminated string literal"),
                    });
                    return None
                }
            }
        }

        return Some(Value::String(ret));

        fn escape(me: &mut Parser, pos: uint, multiline: bool) -> Option<char> {
            match me.cur.next() {
                Some((_, 'b')) => Some('\u0008'),
                Some((_, 't')) => Some('\u0009'),
                Some((_, 'n')) => Some('\u000a'),
                Some((_, 'f')) => Some('\u000c'),
                Some((_, 'r')) => Some('\u000d'),
                Some((_, '"')) => Some('\u0022'),
                Some((_, '/')) => Some('\u002f'),
                Some((_, '\\')) => Some('\u005c'),
                Some((pos, c @ 'u')) |
                Some((pos, c @ 'U')) => {
                    let len = if c == 'u' {4} else {8};
                    let num = if me.input.is_char_boundary(pos + 1 + len) {
                        me.input.slice(pos + 1, pos + 1 + len)
                    } else {
                        "invalid"
                    };
                    match FromStrRadix::from_str_radix(num, 16) {
                        Some(n) => {
                            match char::from_u32(n) {
                                Some(c) => {
                                    for _ in range(0, len) {
                                        me.cur.next();
                                    }
                                    return Some(c)
                                }
                                None => {
                                    me.errors.push(ParserError {
                                        lo: pos + 1,
                                        hi: pos + 5,
                                        desc: format!("codepoint `{:x}` is \
                                                       not a valid unicode \
                                                       codepoint", n),
                                    })
                                }
                            }
                        }
                        None => {
                            me.errors.push(ParserError {
                                lo: pos,
                                hi: pos + 1,
                                desc: format!("expected {} hex digits \
                                               after a `{}` escape", len, c),
                            })
                        }
                    }
                    None
                }
                Some((_, '\n')) if multiline => {
                    loop {
                        match me.cur.clone().next() {
                            Some((_, '\t')) |
                            Some((_, ' ')) |
                            Some((_, '\n')) => { me.cur.next(); }
                            _ => break
                        }
                    }
                    None
                }
                Some((pos, ch)) => {
                    let mut escaped = String::new();
                    ch.escape_default(|c| escaped.push(c));
                    let next_pos = me.next_pos();
                    me.errors.push(ParserError {
                        lo: pos,
                        hi: next_pos,
                        desc: format!("unknown string escape: `{}`",
                                      escaped),
                    });
                    None
                }
                None => {
                    me.errors.push(ParserError {
                        lo: pos,
                        hi: pos + 1,
                        desc: format!("unterminated escape sequence"),
                    });
                    None
                }
            }
        }
    }

    fn literal_string(&mut self, start: uint) -> Option<Value> {
        if !self.expect('\'') { return None }
        let mut multiline = false;
        let mut ret = String::new();

        // detect multiline literals
        if self.eat('\'') {
            multiline = true;
            if !self.expect('\'') { return None }
            self.eat('\n');
        }

        loop {
            match self.cur.next() {
                Some((_, '\'')) => {
                    if multiline {
                        if !self.eat('\'') { ret.push_str("'"); continue }
                        if !self.eat('\'') { ret.push_str("''"); continue }
                    }
                    break
                }
                Some((_, ch)) => ret.push(ch),
                None => {
                    self.errors.push(ParserError {
                        lo: start,
                        hi: self.input.len(),
                        desc: format!("unterminated string literal"),
                    });
                    return None
                }
            }
        }

        return Some(Value::String(ret));
    }

    fn number_or_datetime(&mut self, start: uint) -> Option<Value> {
        let negative = self.eat('-');
        let mut is_float = false;
        loop {
            match self.cur.clone().next() {
                Some((_, ch)) if ch.is_digit() => { self.cur.next(); }
                Some((_, '.')) if !is_float => {
                    is_float = true;
                    self.cur.next();
                }
                Some(_) | None => break,
            }
        }
        let end = self.next_pos();
        let ret = if is_float {
            if self.input.char_at_reverse(end) == '.' {
                None
            } else {
                from_str::<f64>(self.input.slice(start, end)).map(Float)
            }
        } else if !negative && self.eat('-') {
            self.datetime(start, end + 1)
        } else {
            from_str::<i64>(self.input.slice(start, end)).map(Integer)
        };
        if ret.is_none() {
            self.errors.push(ParserError {
                lo: start,
                hi: end,
                desc: format!("invalid numeric literal"),
            });
        }
        return ret;
    }

    fn boolean(&mut self, start: uint) -> Option<Value> {
        let rest = self.input.slice_from(start);
        if rest.starts_with("true") {
            for _ in range(0u, 4u) {
                self.cur.next();
            }
            Some(Boolean(true))
        } else if rest.starts_with("false") {
            for _ in range(0u, 5u) {
                self.cur.next();
            }
            Some(Boolean(false))
        } else {
            let next = self.next_pos();
            self.errors.push(ParserError {
                lo: start,
                hi: next,
                desc: format!("unexpected character: `{}`",
                             rest.char_at(0)),
            });
            None
        }
    }

    fn datetime(&mut self, start: uint, end_so_far: uint) -> Option<Value> {
        let mut date = self.input.slice(start, end_so_far).to_string();
        for _ in range(0u, 15u) {
            match self.cur.next() {
                Some((_, ch)) => date.push(ch),
                None => {
                    self.errors.push(ParserError {
                        lo: start,
                        hi: end_so_far,
                        desc: format!("malformed date literal"),
                    });
                    return None
                }
            }
        }
        let mut it = date.as_slice().chars();
        let mut valid = true;
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == 'T').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit()).unwrap_or(false);
        valid = valid && it.next().map(|c| c == 'Z').unwrap_or(false);
        if valid {
            Some(Datetime(date.clone()))
        } else {
            self.errors.push(ParserError {
                lo: start,
                hi: start + date.len(),
                desc: format!("malformed date literal"),
            });
            None
        }
    }

    fn array(&mut self, _start: uint) -> Option<Value> {
        if !self.expect('[') { return None }
        let mut ret = Vec::new();
        fn consume(me: &mut Parser) {
            loop {
                me.ws();
                match me.cur.clone().next() {
                    Some((_, '#')) => { me.comment(); }
                    Some((_, '\n')) |
                    Some((_, '\r')) => { me.cur.next(); }
                    _ => break,
                }
            }
        }
        let mut type_str = None;
        loop {
            // Break out early if we see the closing bracket
            consume(self);
            if self.eat(']') { return Some(Array(ret)) }

            // Attempt to parse a value, triggering an error if it's the wrong
            // type.
            let start = self.next_pos();
            let value = match self.value() {
                Some(v) => v,
                None => return None,
            };
            let end = self.next_pos();
            let expected = type_str.unwrap_or(value.type_str());
            if value.type_str() != expected {
                self.errors.push(ParserError {
                    lo: start,
                    hi: end,
                    desc: format!("expected type `{}`, found type `{}`",
                                  expected, value.type_str()),
                });
            } else {
                type_str = Some(expected);
                ret.push(value);
            }

            // Look for a comma. If we don't find one we're done
            consume(self);
            if !self.eat(',') { break }
        }
        consume(self);
        if !self.expect(']') { return None }
        return Some(Array(ret))
    }

    fn insert(&mut self, into: &mut TomlTable, key: String, value: Value,
              key_lo: uint) {
        if into.contains_key(&key) {
            self.errors.push(ParserError {
                lo: key_lo,
                hi: key_lo + key.len(),
                desc: format!("duplicate key: `{}`", key),
            })
        } else {
            into.insert(key, value);
        }
    }

    fn recurse<'a>(&mut self, mut cur: &'a mut TomlTable, orig_key: &'a str,
                   key_lo: uint) -> Option<(&'a mut TomlTable, &'a str)> {
        if orig_key.starts_with(".") || orig_key.ends_with(".") ||
           orig_key.contains("..") {
            self.errors.push(ParserError {
                lo: key_lo,
                hi: key_lo + orig_key.len(),
                desc: format!("tables cannot have empty names"),
            });
            return None
        }
        let key = match orig_key.rfind('.') {
            Some(n) => orig_key.slice_to(n),
            None => return Some((cur, orig_key)),
        };
        for part in key.as_slice().split('.') {
            let part = part.to_string();
            let tmp = cur;

            if tmp.contains_key(&part) {
                match *tmp.get_mut(&part).unwrap() {
                    Table(ref mut table) => {
                        cur = table;
                        continue
                    }
                    Array(ref mut array) => {
                        match array.as_mut_slice().last_mut() {
                            Some(&Table(ref mut table)) => cur = table,
                            _ => {
                                self.errors.push(ParserError {
                                    lo: key_lo,
                                    hi: key_lo + key.len(),
                                    desc: format!("array `{}` does not contain \
                                                   tables", part)
                                });
                                return None
                            }
                        }
                        continue
                    }
                    _ => {
                        self.errors.push(ParserError {
                            lo: key_lo,
                            hi: key_lo + key.len(),
                            desc: format!("key `{}` was not previously a table",
                                          part)
                        });
                        return None
                    }
                }
            }

            // Initialize an empty table as part of this sub-key
            tmp.insert(part.clone(), Table(TreeMap::new()));
            match *tmp.get_mut(&part).unwrap() {
                Table(ref mut inner) => cur = inner,
                _ => unreachable!(),
            }
        }
        return Some((cur, orig_key.slice_from(key.len() + 1)))
    }

    fn insert_table(&mut self, into: &mut TomlTable, key: String, value: TomlTable,
                    key_lo: uint) {
        if !self.tables_defined.insert(key.clone()) {
            self.errors.push(ParserError {
                lo: key_lo,
                hi: key_lo + key.len(),
                desc: format!("redefinition of table `{}`", key),
            });
            return
        }

        let (into, key) = match self.recurse(into, key.as_slice(), key_lo) {
            Some(pair) => pair,
            None => return,
        };
        let key = key.to_string();
        if !into.contains_key(&key) {
            into.insert(key.clone(), Table(TreeMap::new()));
        }
        match into.get_mut(&key) {
            Some(&Table(ref mut table)) => {
                for (k, v) in value.into_iter() {
                    if table.insert(k.clone(), v).is_some() {
                        self.errors.push(ParserError {
                            lo: key_lo,
                            hi: key_lo + key.len(),
                            desc: format!("duplicate key `{}` in table", k),
                        });
                    }
                }
            }
            Some(_) => {
                self.errors.push(ParserError {
                    lo: key_lo,
                    hi: key_lo + key.len(),
                    desc: format!("duplicate key `{}` in table", key),
                });
            }
            None => {}
        }
    }

    fn insert_array(&mut self, into: &mut TomlTable, key: String, value: Value,
                   key_lo: uint) {
        let (into, key) = match self.recurse(into, key.as_slice(), key_lo) {
            Some(pair) => pair,
            None => return,
        };
        let key = key.to_string();
        if !into.contains_key(&key) {
            into.insert(key.clone(), Array(Vec::new()));
        }
        match *into.get_mut(&key).unwrap() {
            Array(ref mut vec) => {
                match vec.as_slice().head() {
                    Some(ref v) if !v.same_type(&value) => {
                        self.errors.push(ParserError {
                            lo: key_lo,
                            hi: key_lo + key.len(),
                            desc: format!("expected type `{}`, found type `{}`",
                                          v.type_str(), value.type_str()),
                        })
                    }
                    Some(..) | None => {}
                }
                vec.push(value);
            }
            _ => {
                self.errors.push(ParserError {
                    lo: key_lo,
                    hi: key_lo + key.len(),
                    desc: format!("key `{}` was previously not an array", key),
                });
            }
        }
    }
}

impl Error for ParserError {
    fn description(&self) -> &str { "TOML parse error" }
    fn detail(&self) -> Option<String> { Some(self.desc.clone()) }
}

#[cfg(test)]
mod tests {
    use Value::Table;
    use Parser;

    #[test]
    fn crlf() {
        let mut p = Parser::new("\
[project]\r\n\
\r\n\
name = \"splay\"\r\n\
version = \"0.1.0\"\r\n\
authors = [\"alex@crichton.co\"]\r\n\
\r\n\
[[lib]]\r\n\
\r\n\
path = \"lib.rs\"\r\n\
name = \"splay\"\r\n\
description = \"\"\"\
A Rust implementation of a TAR file reader and writer. This library does not\r\n\
currently handle compression, but it is abstract over all I/O readers and\r\n\
writers. Additionally, great lengths are taken to ensure that the entire\r\n\
contents are never required to be entirely resident in memory all at once.\r\n\
\"\"\"\
");
        assert!(p.parse().is_some());
    }

    #[test]
    fn linecol() {
        let p = Parser::new("ab\ncde\nf");
        assert_eq!(p.to_linecol(0), (0, 0));
        assert_eq!(p.to_linecol(1), (0, 1));
        assert_eq!(p.to_linecol(3), (1, 0));
        assert_eq!(p.to_linecol(4), (1, 1));
        assert_eq!(p.to_linecol(7), (2, 0));
    }

    #[test]
    fn fun_with_strings() {
        let mut p = Parser::new(r#"
bar = "\U00000000"
key1 = "One\nTwo"
key2 = """One\nTwo"""
key3 = """
One
Two"""

key4 = "The quick brown fox jumps over the lazy dog."
key5 = """
The quick brown \


  fox jumps over \
    the lazy dog."""
key6 = """\
       The quick brown \
       fox jumps over \
       the lazy dog.\
       """
# What you see is what you get.
winpath  = 'C:\Users\nodejs\templates'
winpath2 = '\\ServerX\admin$\system32\'
quoted   = 'Tom "Dubs" Preston-Werner'
regex    = '<\i\c*\s*>'

regex2 = '''I [dw]on't need \d{2} apples'''
lines  = '''
The first newline is
trimmed in raw strings.
   All other whitespace
   is preserved.
'''
"#);
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("bar").and_then(|k| k.as_str()), Some("\0"));
        assert_eq!(table.lookup("key1").and_then(|k| k.as_str()),
                   Some("One\nTwo"));
        assert_eq!(table.lookup("key2").and_then(|k| k.as_str()),
                   Some("One\nTwo"));
        assert_eq!(table.lookup("key3").and_then(|k| k.as_str()),
                   Some("One\nTwo"));

        let msg = "The quick brown fox jumps over the lazy dog.";
        assert_eq!(table.lookup("key4").and_then(|k| k.as_str()), Some(msg));
        assert_eq!(table.lookup("key5").and_then(|k| k.as_str()), Some(msg));
        assert_eq!(table.lookup("key6").and_then(|k| k.as_str()), Some(msg));

        assert_eq!(table.lookup("winpath").and_then(|k| k.as_str()),
                   Some(r"C:\Users\nodejs\templates"));
        assert_eq!(table.lookup("winpath2").and_then(|k| k.as_str()),
                   Some(r"\\ServerX\admin$\system32\"));
        assert_eq!(table.lookup("quoted").and_then(|k| k.as_str()),
                   Some(r#"Tom "Dubs" Preston-Werner"#));
        assert_eq!(table.lookup("regex").and_then(|k| k.as_str()),
                   Some(r"<\i\c*\s*>"));
        assert_eq!(table.lookup("regex2").and_then(|k| k.as_str()),
                   Some(r"I [dw]on't need \d{2} apples"));
        assert_eq!(table.lookup("lines").and_then(|k| k.as_str()),
                   Some("The first newline is\n\
                         trimmed in raw strings.\n   \
                            All other whitespace\n   \
                            is preserved.\n"));
    }
}
