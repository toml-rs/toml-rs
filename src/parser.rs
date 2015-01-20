use std::char;
use std::collections::BTreeMap;
use std::error::Error;
use std::num::FromStrRadix;
use std::str;

use Table as TomlTable;
use Value::{self, Array, Table, Float, Integer, Boolean, Datetime};

/// Parser for converting a string to a TOML `Value` instance.
///
/// This parser contains the string slice that is being parsed, and exports the
/// list of errors which have occurred during parsing.
pub struct Parser<'a> {
    input: &'a str,
    cur: str::CharIndices<'a>,

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
#[derive(Show)]
pub struct ParserError {
    /// The low byte at which this error is pointing at.
    pub lo: usize,
    /// One byte beyond the last character at which this error is pointing at.
    pub hi: usize,
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
    ///     Some(value) => println!("found toml: {:?}", value),
    ///     None => {
    ///         println!("parse errors: {:?}", parser.errors);
    ///     }
    /// }
    /// ```
    pub fn new(s: &'a str) -> Parser<'a> {
        Parser {
            input: s,
            cur: s.char_indices(),
            errors: Vec::new(),
        }
    }

    /// Converts a byte offset from an error message to a (line, column) pair
    ///
    /// All indexes are 0-based.
    pub fn to_linecol(&self, offset: usize) -> (usize, usize) {
        let mut cur = 0;
        for (i, line) in self.input.lines().enumerate() {
            if cur + line.len() > offset {
                return (i, offset - cur)
            }
            cur += line.len() + 1;
        }
        return (self.input.lines().count(), 0)
    }

    fn next_pos(&self) -> usize {
        self.cur.clone().next().map(|p| p.0).unwrap_or(self.input.len())
    }

    // Returns true and consumes the next character if it matches `ch`,
    // otherwise do nothing and return false
    fn eat(&mut self, ch: char) -> bool {
        match self.peek(0) {
            Some((_, c)) if c == ch => { self.cur.next(); true }
            Some(_) | None => false,
        }
    }

    // Peeks ahead `n` characters
    fn peek(&self, n: usize) -> Option<(usize, char)> {
        self.cur.clone().skip(n).next()
    }

    fn expect(&mut self, ch: char) -> bool {
        if self.eat(ch) { return true }
        let mut it = self.cur.clone();
        let lo = it.next().map(|p| p.0).unwrap_or(self.input.len());
        let hi = it.next().map(|p| p.0).unwrap_or(self.input.len());
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

    // Consumes whitespace ('\t' and ' ') until another character (or EOF) is
    // reached. Returns if any whitespace was consumed
    fn ws(&mut self) -> bool {
        let mut ret = false;
        loop {
            match self.peek(0) {
                Some((_, '\t')) |
                Some((_, ' ')) => { self.cur.next(); ret = true; }
                _ => break,
            }
        }
        ret
    }

    // Consumes the rest of the line after a comment character
    fn comment(&mut self) -> bool {
        if !self.eat('#') { return false }
        for (_, ch) in self.cur {
            if ch == '\n' { break }
        }
        true
    }

    // Consumes a newline if one is next
    fn newline(&mut self) -> bool {
        match self.peek(0) {
            Some((_, '\n')) => { self.cur.next(); true }
            Some((_, '\r')) if self.peek(1).map(|c| c.1) == Some('\n') => {
                self.cur.next(); self.cur.next(); true
            }
            _ => false
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
        let mut ret = BTreeMap::new();
        while self.peek(0).is_some() {
            self.ws();
            if self.newline() { continue }
            if self.comment() { continue }
            if self.eat('[') {
                let array = self.eat('[');
                let start = self.next_pos();

                // Parse the name of the section
                let mut keys = Vec::new();
                loop {
                    self.ws();
                    match self.key_name() {
                        Some(s) => keys.push(s),
                        None => {}
                    }
                    self.ws();
                    if self.eat(']') {
                        if array && !self.expect(']') { return None }
                        break
                    }
                    if !self.expect('.') { return None }
                }
                if keys.len() == 0 { return None }

                // Build the section table
                let mut table = BTreeMap::new();
                if !self.values(&mut table) { return None }
                if array {
                    self.insert_array(&mut ret, &keys[], Table(table), start)
                } else {
                    self.insert_table(&mut ret, &keys[], table, start)
                }
            } else {
                if !self.values(&mut ret) { return None }
            }
        }
        if self.errors.len() > 0 {
            None
        } else {
            Some(ret)
        }
    }

    // Parse a single key name starting at `start`
    fn key_name(&mut self) -> Option<String> {
        let start = self.next_pos();
        let key = if self.eat('"') {
            self.finish_string(start, false)
        } else {
            let mut ret = String::new();
            loop {
                match self.cur.clone().next() {
                    Some((_, ch)) => {
                        match ch {
                            'a' ... 'z' |
                            'A' ... 'Z' |
                            '0' ... '9' |
                            '_' | '-' => { self.cur.next(); ret.push(ch) }
                            _ => break,
                        }
                    }
                    None => break
                }
            }
            Some(ret)
        };
        match key {
            Some(ref name) if name.len() == 0 => {
                self.errors.push(ParserError {
                    lo: start,
                    hi: start,
                    desc: format!("expected a key but found an empty string"),
                });
                None
            }
            Some(name) => Some(name),
            None => None,
        }
    }

    // Parses the values into the given TomlTable. Returns true in case of success
    // and false in case of error.
    fn values(&mut self, into: &mut TomlTable) -> bool {
        loop {
            self.ws();
            if self.newline() { continue }
            if self.comment() { continue }
            match self.peek(0) {
                Some((_, '[')) => break,
                Some(..) => {}
                None => break,
            }
            let key_lo = self.next_pos();
            let key = match self.key_name() {
                Some(s) => s,
                None => return false
            };
            self.ws();
            if !self.expect('=') { return false }
            let value = match self.value() {
                Some(value) => value,
                None => return false,
            };
            self.insert(into, key, value, key_lo);
            self.ws();
            self.comment();
            self.newline();
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
            Some((pos, '-')) |
            Some((pos, '+')) => self.number_or_datetime(pos),
            Some((pos, ch)) if ch.is_digit(10) => self.number_or_datetime(pos),
            _ => {
                let mut it = self.cur.clone();
                let lo = it.next().map(|p| p.0).unwrap_or(self.input.len());
                let hi = it.next().map(|p| p.0).unwrap_or(self.input.len());
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
    fn string(&mut self, start: usize) -> Option<Value> {
        if !self.expect('"') { return None }
        let mut multiline = false;

        // detect multiline literals, but be careful about empty ""
        // strings
        if self.eat('"') {
            if self.eat('"') {
                multiline = true;
                self.newline();
            } else {
                // empty
                return Some(Value::String(String::new()))
            }
        }

        self.finish_string(start, multiline).map(Value::String)
    }

    // Finish parsing a basic string after the opening quote has been seen
    fn finish_string(&mut self,
                     start: usize,
                     multiline: bool) -> Option<String> {
        let mut ret = String::new();
        loop {
            while multiline && self.newline() { ret.push('\n') }
            match self.cur.next() {
                Some((_, '"')) => {
                    if multiline {
                        if !self.eat('"') { ret.push_str("\""); continue }
                        if !self.eat('"') { ret.push_str("\"\""); continue }
                    }
                    return Some(ret)
                }
                Some((pos, '\\')) => {
                    match escape(self, pos, multiline) {
                        Some(c) => ret.push(c),
                        None => {}
                    }
                }
                Some((pos, ch)) if ch < '\u{1f}' => {
                    self.errors.push(ParserError {
                        lo: pos,
                        hi: pos + 1,
                        desc: format!("control character `{}` must be escaped",
                                      ch.escape_default().collect::<String>())
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

        fn escape(me: &mut Parser, pos: usize, multiline: bool) -> Option<char> {
            if multiline && me.newline() {
                while me.ws() || me.newline() { /* ... */ }
                return None
            }
            match me.cur.next() {
                Some((_, 'b')) => Some('\u{8}'),
                Some((_, 't')) => Some('\u{9}'),
                Some((_, 'n')) => Some('\u{a}'),
                Some((_, 'f')) => Some('\u{c}'),
                Some((_, 'r')) => Some('\u{d}'),
                Some((_, '"')) => Some('\u{22}'),
                Some((_, '\\')) => Some('\u{5c}'),
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
                                    me.cur.by_ref().skip(len - 1).next();
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
                Some((pos, ch)) => {
                    let next_pos = me.next_pos();
                    me.errors.push(ParserError {
                        lo: pos,
                        hi: next_pos,
                        desc: format!("unknown string escape: `{}`",
                                      ch.escape_default().collect::<String>()),
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

    fn literal_string(&mut self, start: usize) -> Option<Value> {
        if !self.expect('\'') { return None }
        let mut multiline = false;
        let mut ret = String::new();

        // detect multiline literals
        if self.eat('\'') {
            if self.eat('\'') {
                multiline = true;
                self.newline();
            } else {
                return Some(Value::String(ret)) // empty
            }
        }

        loop {
            if !multiline && self.newline() {
                let next = self.next_pos();
                self.errors.push(ParserError {
                    lo: start,
                    hi: next,
                    desc: format!("literal strings cannot contain newlines"),
                });
                return None
            }
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

    fn number_or_datetime(&mut self, start: usize) -> Option<Value> {
        let mut is_float = false;
        if !self.integer(start, false, true) { return None }
        if self.eat('.') {
            is_float = true;
            if !self.integer(start, true, false) { return None }
        }
        if self.eat('e') || self.eat('E') {
            is_float = true;
            if !self.integer(start, false, true) { return None }
        }
        let end = self.next_pos();
        let input = self.input.slice(start, end);
        let ret = if !is_float && !input.starts_with("+") &&
                     !input.starts_with("-") && self.eat('-') {
            self.datetime(start, end + 1)
        } else {
            let input = input.trim_left_matches('+');
            if is_float {
                input.parse().map(Float)
            } else {
                input.parse().map(Integer)
            }
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

    fn integer(&mut self, start: usize, allow_leading_zeros: bool,
               allow_sign: bool) -> bool {
        allow_sign && (self.eat('-') || self.eat('+'));
        match self.cur.next() {
            Some((_, '0')) if !allow_leading_zeros => {
                match self.peek(0) {
                    Some((pos, c)) if '0' <= c && c <= '9' => {
                        self.errors.push(ParserError {
                            lo: start,
                            hi: pos,
                            desc: format!("leading zeroes are not allowed"),
                        });
                        return false
                    }
                    _ => {}
                }
            }
            Some((_, ch)) if '0' <= ch && ch <= '9' => {}
            _ => {
                let pos = self.next_pos();
                self.errors.push(ParserError {
                    lo: pos,
                    hi: pos,
                    desc: format!("expected start of a numeric literal"),
                });
                return false;
            }
        }
        loop {
            match self.cur.clone().next() {
                Some((_, ch)) if '0' <= ch && ch <= '9' => { self.cur.next(); }
                Some(_) | None => break,
            }
        }
        true
    }

    fn boolean(&mut self, start: usize) -> Option<Value> {
        let rest = self.input.slice_from(start);
        if rest.starts_with("true") {
            for _ in 0..4 {
                self.cur.next();
            }
            Some(Boolean(true))
        } else if rest.starts_with("false") {
            for _ in 0..5 {
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

    fn datetime(&mut self, start: usize, end_so_far: usize) -> Option<Value> {
        let mut date = self.input.slice(start, end_so_far).to_string();
        for _ in 0..15 {
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
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c == 'T').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
        valid = valid && it.next().map(|c| c.is_digit(10)).unwrap_or(false);
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

    fn array(&mut self, _start: usize) -> Option<Value> {
        if !self.expect('[') { return None }
        let mut ret = Vec::new();
        fn consume(me: &mut Parser) {
            loop {
                me.ws();
                if !me.newline() && !me.comment() { break }
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
              key_lo: usize) {
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

    fn recurse<'b>(&mut self, mut cur: &'b mut TomlTable, keys: &'b [String],
                   key_lo: usize) -> Option<(&'b mut TomlTable, &'b str)> {
        let key_hi = keys.iter().fold(0, |a, b| a + b.len());
        for part in keys[..keys.len() - 1].iter() {
            let tmp = cur;

            if tmp.contains_key(part) {
                match *tmp.get_mut(part).unwrap() {
                    Table(ref mut table) => {
                        cur = table;
                        continue
                    }
                    Array(ref mut array) => {
                        match array.last_mut() {
                            Some(&mut Table(ref mut table)) => cur = table,
                            _ => {
                                self.errors.push(ParserError {
                                    lo: key_lo,
                                    hi: key_hi,
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
                            hi: key_hi,
                            desc: format!("key `{}` was not previously a table",
                                          part)
                        });
                        return None
                    }
                }
            }

            // Initialize an empty table as part of this sub-key
            tmp.insert(part.clone(), Table(BTreeMap::new()));
            match *tmp.get_mut(part).unwrap() {
                Table(ref mut inner) => cur = inner,
                _ => unreachable!(),
            }
        }
        Some((cur, &keys.last().unwrap()[]))
    }

    fn insert_table(&mut self, into: &mut TomlTable, keys: &[String],
                    value: TomlTable, key_lo: usize) {
        let (into, key) = match self.recurse(into, keys, key_lo) {
            Some(pair) => pair,
            None => return,
        };
        let key = key.to_string();
        let mut added = false;
        if !into.contains_key(&key) {
            into.insert(key.clone(), Table(BTreeMap::new()));
            added = true;
        }
        match into.get_mut(&key) {
            Some(&mut Table(ref mut table)) => {
                let any_tables = table.values().any(|v| v.as_table().is_some());
                if !any_tables && !added {
                    self.errors.push(ParserError {
                        lo: key_lo,
                        hi: key_lo + key.len(),
                        desc: format!("redefinition of table `{}`", key),
                    });
                }
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

    fn insert_array(&mut self, into: &mut TomlTable,
                    keys: &[String], value: Value, key_lo: usize) {
        let (into, key) = match self.recurse(into, keys, key_lo) {
            Some(pair) => pair,
            None => return,
        };
        let key = key.to_string();
        if !into.contains_key(&key) {
            into.insert(key.clone(), Array(Vec::new()));
        }
        match *into.get_mut(&key).unwrap() {
            Array(ref mut vec) => {
                match vec.as_slice().first() {
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

    #[test]
    fn tables_in_arrays() {
        let mut p = Parser::new(r#"
[[foo]]
  #…
  [foo.bar]
    #…

[[foo]]
  #…
  [foo.bar]
    #...
"#);
        let table = Table(p.parse().unwrap());
        table.lookup("foo.0.bar").unwrap().as_table().unwrap();
        table.lookup("foo.1.bar").unwrap().as_table().unwrap();
    }

    #[test]
    fn fruit() {
        let mut p = Parser::new(r#"
[[fruit]]
  name = "apple"

  [fruit.physical]
    color = "red"
    shape = "round"

  [[fruit.variety]]
    name = "red delicious"

  [[fruit.variety]]
    name = "granny smith"

[[fruit]]
  name = "banana"

  [[fruit.variety]]
    name = "plantain"
"#);
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("fruit.0.name").and_then(|k| k.as_str()),
                   Some("apple"));
        assert_eq!(table.lookup("fruit.0.physical.color").and_then(|k| k.as_str()),
                   Some("red"));
        assert_eq!(table.lookup("fruit.0.physical.shape").and_then(|k| k.as_str()),
                   Some("round"));
        assert_eq!(table.lookup("fruit.0.variety.0.name").and_then(|k| k.as_str()),
                   Some("red delicious"));
        assert_eq!(table.lookup("fruit.0.variety.1.name").and_then(|k| k.as_str()),
                   Some("granny smith"));
        assert_eq!(table.lookup("fruit.1.name").and_then(|k| k.as_str()),
                   Some("banana"));
        assert_eq!(table.lookup("fruit.1.variety.0.name").and_then(|k| k.as_str()),
                   Some("plantain"));
    }

    #[test]
    fn stray_cr() {
        assert!(Parser::new("\r").parse().is_none());
        assert!(Parser::new("a = [ \r ]").parse().is_none());
        assert!(Parser::new("a = \"\"\"\r\"\"\"").parse().is_none());
        assert!(Parser::new("a = \"\"\"\\  \r  \"\"\"").parse().is_none());

        let mut p = Parser::new("foo = '''\r'''");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").and_then(|k| k.as_str()), Some("\r"));

        let mut p = Parser::new("foo = '\r'");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").and_then(|k| k.as_str()), Some("\r"));
    }

    #[test]
    fn blank_literal_string() {
        let mut p = Parser::new("foo = ''");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").and_then(|k| k.as_str()), Some(""));
    }

    #[test]
    fn many_blank() {
        let mut p = Parser::new("foo = \"\"\"\n\n\n\"\"\"");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").and_then(|k| k.as_str()), Some("\n\n"));
    }

    #[test]
    fn literal_eats_crlf() {
        let mut p = Parser::new("
            foo = \"\"\"\\\r\n\"\"\"
            bar = \"\"\"\\\r\n   \r\n   \r\n   a\"\"\"
        ");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").and_then(|k| k.as_str()), Some(""));
        assert_eq!(table.lookup("bar").and_then(|k| k.as_str()), Some("a"));
    }

    #[test]
    fn string_no_newline() {
        assert!(Parser::new("a = \"\n\"").parse().is_none());
        assert!(Parser::new("a = '\n'").parse().is_none());
    }

    #[test]
    fn bad_leading_zeros() {
        assert!(Parser::new("a = 00").parse().is_none());
        assert!(Parser::new("a = -00").parse().is_none());
        assert!(Parser::new("a = +00").parse().is_none());
        assert!(Parser::new("a = 00.0").parse().is_none());
        assert!(Parser::new("a = -00.0").parse().is_none());
        assert!(Parser::new("a = +00.0").parse().is_none());
        assert!(Parser::new("a = 9223372036854775808").parse().is_none());
        assert!(Parser::new("a = -9223372036854775809").parse().is_none());
    }

    #[test]
    fn bad_floats() {
        assert!(Parser::new("a = 0.").parse().is_none());
        assert!(Parser::new("a = 0.e").parse().is_none());
        assert!(Parser::new("a = 0.E").parse().is_none());
        assert!(Parser::new("a = 0.0E").parse().is_none());
        assert!(Parser::new("a = 0.0e").parse().is_none());
        assert!(Parser::new("a = 0.0e-").parse().is_none());
        assert!(Parser::new("a = 0.0e+").parse().is_none());
        assert!(Parser::new("a = 0.0e+00").parse().is_none());
    }

    #[test]
    fn floats() {
        macro_rules! t {
            ($actual:expr, $expected:expr) => ({
                let f = format!("foo = {}", $actual);
                let mut p = Parser::new(&f[]);
                let table = Table(p.parse().unwrap());
                assert_eq!(table.lookup("foo").and_then(|k| k.as_float()),
                           Some($expected));
            })
        }

        t!("1.0", 1.0);
        t!("1.0e0", 1.0);
        t!("1.0e+0", 1.0);
        t!("1.0e-0", 1.0);
        t!("1.001e-0", 1.001);
        t!("2e10", 2e10);
        t!("2e+10", 2e10);
        t!("2e-10", 2e-10);
    }

    #[test]
    fn bare_key_names() {
        let mut p = Parser::new("
            foo = 3
            foo_3 = 3
            foo_-2--3--r23f--4-f2-4 = 3
            _ = 3
            - = 3
            8 = 8
            \"a\" = 3
            \"!\" = 3
            \"a^b\" = 3
            \"\\\"\" = 3
            \"character encoding\" = \"value\"
            \"ʎǝʞ\" = \"value\"
        ");
        let table = Table(p.parse().unwrap());
        assert!(table.lookup("foo").is_some());
        assert!(table.lookup("-").is_some());
        assert!(table.lookup("_").is_some());
        assert!(table.lookup("8").is_some());
        assert!(table.lookup("foo_3").is_some());
        assert!(table.lookup("foo_-2--3--r23f--4-f2-4").is_some());
        assert!(table.lookup("a").is_some());
        assert!(table.lookup("!").is_some());
        assert!(table.lookup("\"").is_some());
        assert!(table.lookup("character encoding").is_some());
        assert!(table.lookup("ʎǝʞ").is_some());
    }

    #[test]
    fn bad_keys() {
        assert!(Parser::new("key\n=3").parse().is_none());
        assert!(Parser::new("key=\n3").parse().is_none());
        assert!(Parser::new("key|=3").parse().is_none());
        assert!(Parser::new("\"\"=3").parse().is_none());
        assert!(Parser::new("=3").parse().is_none());
        assert!(Parser::new("\"\"|=3").parse().is_none());
        assert!(Parser::new("\"\n\"|=3").parse().is_none());
        assert!(Parser::new("\"\r\"|=3").parse().is_none());
    }

    #[test]
    fn bad_table_names() {
        assert!(Parser::new("[]").parse().is_none());
        assert!(Parser::new("[.]").parse().is_none());
        assert!(Parser::new("[\"\".\"\"]").parse().is_none());
        assert!(Parser::new("[a.]").parse().is_none());
        assert!(Parser::new("[\"\"]").parse().is_none());
        assert!(Parser::new("[!]").parse().is_none());
        assert!(Parser::new("[\"\n\"]").parse().is_none());
        assert!(Parser::new("[a.b]\n[a.\"b\"]").parse().is_none());
    }

    #[test]
    fn table_names() {
        let mut p = Parser::new("
            [a.\"b\"]
            [\"f f\"]
            [\"f.f\"]
            [\"\\\"\"]
        ");
        let table = Table(p.parse().unwrap());
        assert!(table.lookup("a.b").is_some());
        assert!(table.lookup("f f").is_some());
        assert!(table.lookup("\"").is_some());
    }

    #[test]
    fn invalid_bare_numeral() {
        assert!(Parser::new("4").parse().is_none());
    }
}
