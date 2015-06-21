use std::ascii::AsciiExt;
use std::char;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;
use std::str;
use std::cell::{RefCell};
use std::rc::Rc;
use std::mem;

use doc::{ContainerData, Formatted, Key, KvpMap, RootTable};
use doc::{IndirectChild, Container};
use doc::Value as DocValue;

macro_rules! try {
    ($e:expr) => (match $e { Some(s) => s, None => return None })
}

type Segment<'a> =(Option<&'a mut KvpMap>, Option<&'a mut HashMap<String,IndirectChild>>);

/// Parser for converting a string to a TOML `Value` instance.
///
/// This parser contains the string slice that is being parsed, and exports the
/// list of errors which have occurred during parsing.
pub struct Parser<'a> {
    input: &'a str,
    cur: str::CharIndices<'a>,
    aux_text: &'a str,

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
#[derive(Debug)]
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
            aux_text: ""
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

    fn is_at_end(&self) -> bool {
        self.cur.clone().next().is_none()
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
        for (_, ch) in self.cur.by_ref() {
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

    fn skip_aux(&mut self) {
        let start = self.next_pos();
        loop {
            self.ws();
            if self.newline() { continue }
            if self.comment() { continue }
            break;
        }
        self.aux_text = &self.input[start..self.next_pos()];
    }

    fn eat_aux(&mut self) -> &str {
        self.skip_aux();
        self.take_aux()
    }

    fn eat_to_newline(&mut self) -> &str {
        let start = self.next_pos();
        self.ws();
        self.newline();
        self.comment();
        self.aux_text = &self.input[start..self.next_pos()];
        self.take_aux()
    }

    fn eat_ws(&mut self) -> &str {
        self.skip_ws();
        self.take_aux()
    }

    fn skip_ws(&mut self) {
        let start = self.next_pos();
        self.ws();
        self.aux_text = &self.input[start..self.next_pos()];
    }

    fn take_aux<'b>(&'b mut self) -> &'a str {
        let temp = self.aux_text;
        self.aux_text = "";
        temp
    }

    /// Executes the parser, parsing the string contained within.
    ///
    /// This function will return the `TomlTable` instance if parsing is
    /// successful, or it will return `None` if any parse error or invalid TOML
    /// error occurs.
    ///
    /// If an error occurs, the `errors` field of this parser can be consulted
    /// to determine the cause of the parse failure.
    pub fn parse(&mut self) -> Option<super::Table> {
        self.parse_doc().map(|x| x.convert())
    }

    /// TODO: write something here
    pub fn parse_doc(&mut self) -> Option<super::doc::RootTable> {
        let mut ret = RootTable::new();
        ret.lead = self.eat_aux().to_string();
        while self.peek(0).is_some() {
            let container_aux = self.eat_aux().to_string();
            if self.eat('[') {
                let array = self.eat('[');
                let start = self.next_pos();

                // Parse the name of the section
                let mut keys = Vec::new();
                loop {
                    let key_lead_aux = self.eat_ws().to_string();
                    if let Some(s) = self.key_name() {
                        keys.push(Key::new(key_lead_aux, s, self.eat_ws()));
                    }
                    if self.eat(']') {
                        if array && !self.expect(']') { return None }
                        break
                    }
                    if !self.expect('.') { return None }
                }
                if keys.len() == 0 { return None }

                // Build the section table
                let mut container = ContainerData::new();
                let container_trail = match self.values(&mut container.direct) {
                    Some(str_buf) => str_buf,
                    None => return None
                };
                if array {
                    self.insert_array(&mut ret, keys, container, 
                                      container_aux, container_trail)
                } else {
                    self.insert_table(&mut ret, keys, container,
                                      container_aux, container_trail)
                }
            } else {
                if !self.values(&mut ret.values).is_some() { return None }
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
            while let Some((_, ch)) = self.cur.clone().next() {
                match ch {
                    'a' ... 'z' |
                    'A' ... 'Z' |
                    '0' ... '9' |
                    '_' | '-' => { self.cur.next(); ret.push(ch) }
                    _ => break,
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
    fn values(&mut self, into: &mut KvpMap) -> Option<String> {
        loop {
            let pre_key = self.eat_aux().to_string();
            match self.peek(0) {
                Some((_, '[')) => return Some(pre_key),
                Some(..) => {}
                None => return Some(pre_key),
            }
            let key_lo = self.next_pos();
            let key = match self.key_name() {
                Some(s) => s,
                None => return None
            };
            let key = Key::new(pre_key, key, self.eat_ws());
            if !self.expect('=') { return None }
            let value = match self.value() {
                Some(value) => value,
                None => return None,
            };
            self.insert(into, key, value, key_lo);
            into.set_last_value_trail(self.eat_to_newline());
        }
    }

    // Parses a value
    fn value(&mut self) -> Option<Formatted<DocValue>> {
        let leading_ws = self.eat_ws().to_string();
        let value = match self.cur.clone().next() {
            Some((pos, '"')) => self.string(pos),
            Some((pos, '\'')) => self.literal_string(pos),
            Some((pos, 't')) |
            Some((pos, 'f')) => self.boolean(pos),
            Some((pos, '[')) => self.array(pos),
            Some((pos, '{')) => self.inline_table(pos),
            Some((pos, '-')) |
            Some((pos, '+')) => self.number_or_datetime(pos),
            Some((pos, ch)) if is_digit(ch) => self.number_or_datetime(pos),
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
        };
        value.map(|v| Formatted::<DocValue>::new(leading_ws, v))
    }

    // Parses a single or multi-line string
    fn string(&mut self, start: usize) -> Option<DocValue> {
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
                return Some(DocValue::String(String::new()))
            }
        }

        self.finish_string(start, multiline).map(DocValue::String)
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
                    if let Some(c) = escape(self, pos, multiline) {
                        ret.push(c);
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
                    let num = &me.input[pos+1..];
                    let num = if num.len() >= len && num.is_ascii() {
                        &num[..len]
                    } else {
                        "invalid"
                    };
                    if let Some(n) = u32::from_str_radix(num, 16).ok() {
                        if let Some(c) = char::from_u32(n) {
                            me.cur.by_ref().skip(len - 1).next();
                            return Some(c)
                        } else {
                            me.errors.push(ParserError {
                                lo: pos + 1,
                                hi: pos + 5,
                                desc: format!("codepoint `{:x}` is \
                                               not a valid unicode \
                                               codepoint", n),
                            })
                        }
                    } else {
                        me.errors.push(ParserError {
                            lo: pos,
                            hi: pos + 1,
                            desc: format!("expected {} hex digits \
                                           after a `{}` escape", len, c),
                        })
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

    fn literal_string(&mut self, start: usize) -> Option<DocValue> {
        if !self.expect('\'') { return None }
        let mut multiline = false;
        let mut ret = String::new();

        // detect multiline literals
        if self.eat('\'') {
            if self.eat('\'') {
                multiline = true;
                self.newline();
            } else {
                return Some(DocValue::String(ret)) // empty
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

        return Some(DocValue::String(ret));
    }

    fn number_or_datetime(&mut self, start: usize) -> Option<DocValue> {
        let mut is_float = false;
        let prefix = try!(self.integer(start, false, true));
        let decimal = if self.eat('.') {
            is_float = true;
            Some(try!(self.integer(start, true, false)))
        } else {
            None
        };
        let exponent = if self.eat('e') || self.eat('E') {
            is_float = true;
            Some(try!(self.integer(start, false, true)))
        } else {
            None
        };
        let end = self.next_pos();
        let input = &self.input[start..end];
        let ret = if !is_float && !input.starts_with("+") &&
                     !input.starts_with("-") && self.eat('-') {
            self.datetime(start, end + 1)
        } else {
            let input = match (decimal, exponent) {
                (None, None) => prefix,
                (Some(ref d), None) => prefix + "." + d,
                (None, Some(ref e)) => prefix + "E" + e,
                (Some(ref d), Some(ref e)) => prefix + "." + d + "E" + e,
            };
            let input = input.trim_left_matches('+');
            if is_float {
                input.parse().ok().map(DocValue::Float)
            } else {
                input.parse().ok().map(DocValue::Integer)
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
               allow_sign: bool) -> Option<String> {
        let mut s = String::new();
        if allow_sign {
            if self.eat('-') { s.push('-'); }
            else if self.eat('+') { s.push('+'); }
        }
        match self.cur.next() {
            Some((_, '0')) if !allow_leading_zeros => {
                s.push('0');
                match self.peek(0) {
                    Some((pos, c)) if '0' <= c && c <= '9' => {
                        self.errors.push(ParserError {
                            lo: start,
                            hi: pos,
                            desc: format!("leading zeroes are not allowed"),
                        });
                        return None
                    }
                    _ => {}
                }
            }
            Some((_, ch)) if '0' <= ch && ch <= '9' => {
                s.push(ch);
            }
            _ => {
                let pos = self.next_pos();
                self.errors.push(ParserError {
                    lo: pos,
                    hi: pos,
                    desc: format!("expected start of a numeric literal"),
                });
                return None;
            }
        }
        let mut underscore = false;
        loop {
            match self.cur.clone().next() {
                Some((_, ch)) if '0' <= ch && ch <= '9' => {
                    s.push(ch);
                    self.cur.next();
                    underscore = false;
                }
                Some((_, '_')) if !underscore => {
                    self.cur.next();
                    underscore = true;
                }
                Some(_) | None => break,
            }
        }
        if underscore {
            let pos = self.next_pos();
            self.errors.push(ParserError {
                lo: pos,
                hi: pos,
                desc: format!("numeral cannot end with an underscore"),
            });
            return None
        } else {
            Some(s)
        }
    }

    fn boolean(&mut self, start: usize) -> Option<DocValue> {
        let rest = &self.input[start..];
        if rest.starts_with("true") {
            for _ in 0..4 {
                self.cur.next();
            }
            Some(DocValue::Boolean(true))
        } else if rest.starts_with("false") {
            for _ in 0..5 {
                self.cur.next();
            }
            Some(DocValue::Boolean(false))
        } else {
            let next = self.next_pos();
            self.errors.push(ParserError {
                lo: start,
                hi: next,
                desc: format!("unexpected character: `{}`",
                              rest.chars().next().unwrap()),
            });
            None
        }
    }

    fn datetime(&mut self, start: usize, end_so_far: usize) 
                -> Option<DocValue> {
        let mut date = format!("{}", &self.input[start..end_so_far]);
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
        let mut it = date.chars();
        let mut valid = true;
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == '-').unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == 'T').unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == ':').unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(is_digit).unwrap_or(false);
        valid = valid && it.next().map(|c| c == 'Z').unwrap_or(false);
        if valid {
            Some(DocValue::Datetime(date.clone()))
        } else {
            self.errors.push(ParserError {
                lo: start,
                hi: start + date.len(),
                desc: format!("malformed date literal"),
            });
            None
        }
    }

    fn array(&mut self, _start: usize) -> Option<DocValue> {
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
            if self.eat(']') { return Some(DocValue::Array(ret)) }

            // Attempt to parse a value, triggering an error if it's the wrong
            // type.
            let start = self.next_pos();
            let value = try!(self.value());
            let end = self.next_pos();
            let expected = type_str.unwrap_or(value.value.type_str());
            if value.value.type_str() != expected {
                self.errors.push(ParserError {
                    lo: start,
                    hi: end,
                    desc: format!("expected type `{}`, found type `{}`",
                                  expected, value.value.type_str()),
                });
            } else {
                type_str = Some(expected);
                // TODO: handle trailing aux
                ret.push(value);
            }

            // Look for a comma. If we don't find one we're done
            consume(self);
            if !self.eat(',') { break }
        }
        consume(self);
        if !self.expect(']') { return None }
        return Some(DocValue::Array(ret))
    }

    fn inline_table(&mut self, _start: usize) -> Option<DocValue> {
        if !self.expect('{') { return None }
        self.ws();
        let mut ret = KvpMap::new();
        if self.eat('}') { return Some(DocValue::InlineTable(ret)) }
        loop {
            let lo = self.next_pos();
            let key = try!(self.key_name());
            self.ws();
            if !self.expect('=') { return None }
            self.ws();
            let value = try!(self.value());
            self.insert(&mut ret, Key::new(String::new(), key, ""), value, lo);

            self.ws();
            if self.eat('}') { break }
            if !self.expect(',') { return None }
            self.ws();
        }
        return Some(DocValue::InlineTable(ret))
    }

    fn insert(&mut self, into: &mut KvpMap, key: Key,
              value: Formatted<DocValue>, key_lo: usize) {
        let key_text = key.key.clone();
        if !into.insert(key, value) {
            self.errors.push(ParserError {
                lo: key_lo,
                hi: key_lo + key_text.len(),
                desc: format!("duplicate key: `{}`", key_text),
            })
        }
    }

    fn _insert_exec<F, U>(&mut self, cur: Segment, keys: Vec<Key>, idx: usize, f:F)
        -> Option<U>
        where F: FnOnce(&mut Parser, Segment, Vec<Key>) -> U {
        if idx == keys.len() - 1 { Some(f(self, cur, keys)) }
        else {
            if let Some(values) = cur.0 {
                match values.kvp_index.entry(keys[idx].key.clone()) {
                    Entry::Occupied(mut entry) => {
                        match &mut entry.get_mut().borrow_mut().value {
                            &mut DocValue::InlineTable(ref mut c) => {
                                let segment = (Some(c), None);
                                return self._insert_exec(segment, keys, idx+1, f);
                            }
                            &mut DocValue::Array(ref mut vec) => {
                                let has_tables = match vec.first() {
                                    None => false,
                                    Some(v) => v.value.is_table()
                                };
                                if !has_tables {
                                    self.errors.push(ParserError {
                                        lo: 0,
                                        hi: 0,
                                        desc: format!("array `{}` does not contain tables",
                                                       &*keys[idx].key)
                                    });
                                    return None;
                                }
                                let idx_last = vec.len()-1;
                                let c = vec[idx_last].value.as_table();
                                return self._insert_exec((Some(c), None), keys, idx+1, f);
                            }
                            _ => {
                                self.errors.push(ParserError {
                                    lo: 0,
                                    hi: 0,
                                    desc: format!("key `{}` was not previously a table",
                                                   &*keys[idx].key)
                                });
                                return None;
                            }
                        }
                    }
                    Entry::Vacant(_) => { }
                }
            }
            // TODO: fix error message
            if cur.1.is_none() {
                self.errors.push(ParserError {
                    lo: 0,
                    hi: 0,
                    desc: format!("bad things happened")
                });
                return None;
            }
            match cur.1.unwrap().entry(keys[idx].key.clone()) {
                Entry::Occupied(mut entry) => match *entry.get_mut() {
                    IndirectChild::ImplicitTable(ref mut m)
                        => self._insert_exec((None, Some(m)), keys, idx+1, f),
                    IndirectChild::ExplicitTable(ref mut c) => {
                        let c_data = &mut c.borrow_mut().data;
                        let segment =
                            (Some(&mut c_data.direct), Some(&mut c_data.indirect));
                        self._insert_exec(segment, keys, idx+1, f)
                    }
                    IndirectChild::Array(ref mut vec) => {
                        let mut c_data =
                            &mut vec.last().as_mut().unwrap().borrow_mut().data;
                        let segment =
                            (Some(&mut c_data.direct), Some(&mut c_data.indirect));
                        self._insert_exec(segment, keys, idx+1, f)
                    }
                },
                Entry::Vacant(entry) => {
                    let empty = HashMap::new();
                    let map = entry.insert(IndirectChild::ImplicitTable(empty));
                    self._insert_exec((None, Some(map.as_implicit())), keys, idx+1,f)
                }
            }
        }
    }

    fn insert_exec<F, U>(&mut self, r: &mut RootTable, keys: Vec<Key>, f:F)
                         -> Option<U>
                         where F: FnOnce(&mut Parser, Segment, Vec<Key>) -> U {
        self._insert_exec((Some(&mut r.values), Some(&mut r.table_index)), keys, 0, f)
    }

    fn insert_table(&mut self, root: &mut RootTable, keys: Vec<Key>,
                    table: ContainerData, lead: String, trail: String) {
        let added = self.insert_exec(root, keys, |this, seg, keys| {
            { let key = keys.last();
            let key = key.as_ref().unwrap();
            if let Some(map) = seg.0 {
                if map.contains_key(&*key.key) {
                    let is_table = map.kvp_index.get(&*key.key)
                                   .unwrap().borrow().value.is_table();
                    this.errors.push(ParserError {
                        lo: 0,
                        hi: 0,
                        desc: if is_table { format!("redefinition of table `{}`", &*key.key) }
                              else { format!("duplicate key `{}` in table", &*key.key) },
                            
                    });
                    return None;
                }
            }}
            let key_text = keys.last().as_ref().unwrap().key.clone();
            let container = Container::new_table(table, keys, lead, trail);
            let container = Rc::new(RefCell::new(container));
            match seg.1.unwrap().entry(key_text) {
                Entry::Occupied(mut entry) => {
                    let is_implicit = match entry.get() {
                        &IndirectChild::ImplicitTable(_) => true,
                        _ => false
                    };
                    if is_implicit {
                       let old = entry.insert(IndirectChild::ExplicitTable(container.clone()));
                       for (k,v) in old.to_implicit() {
                            let key_copy = k.clone();
                            if container.borrow_mut().data.indirect.insert(k, v).is_some() {
                                this.errors.push(ParserError {
                                    lo: 0,
                                    hi: 0,
                                    desc: format!("duplicate key `{}` in table", key_copy),
                                });
                                return None;
                            }
                       }
                       Some(container)
                    }
                    else {
                        let keys = &container.borrow().keys;
                        this.errors.push(ParserError {
                            lo: 0,
                            hi: 0,
                            desc: format!("redefinition of table `{}`",
                                           &*keys.last().as_ref().unwrap().key),
                        });
                        None
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(IndirectChild::ExplicitTable(container.clone()));
                    Some(container)
                }
            }
        });
        if let Some(ptr) = added.and_then(|x| x) {
            root.table_list.push(ptr);
        }
    }

    fn insert_array(&mut self, root: &mut RootTable, keys: Vec<Key>,
                    table: ContainerData, lead: String, trail: String) {
        let added = self.insert_exec(root, keys, |this, seg, keys| {
            { let key = keys.last();
            let key = key.as_ref().unwrap();
            if let Some(map) = seg.0 {
                if map.contains_key(&*key.key) {
                    this.errors.push(ParserError {
                        lo: 0,
                        hi: 0,
                        desc: format!("duplicate key `{}` in table", &*key.key),
                    });
                    return None;
                }
            }}
            let key_text = keys.last().as_ref().unwrap().key.clone();
            let container = Container::new_array(table, keys, lead, trail);
            let container = Rc::new(RefCell::new(container));
            match seg.1.unwrap().entry(key_text) {
                Entry::Occupied(mut entry) => {
                    match *entry.get_mut() {
                        IndirectChild::ExplicitTable(_)
                        | IndirectChild::ImplicitTable(_) => {
                            let keys = &container.borrow().keys;
                            this.errors.push(ParserError {
                                lo: 0,
                                hi: 0,
                                desc:
                                    format!(
                                        "redefinition of table `{}`",
                                        &*keys.last().as_ref().unwrap().key),
                            });
                            None
                        }
                        IndirectChild::Array(ref mut vec) => {
                            vec.push(container.clone());
                            Some(container)
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(IndirectChild::Array(vec!(container.clone())));
                    Some(container)
                }
            }
        });
        if let Some(ptr) = added.and_then(|x| x) {
            root.table_list.push(ptr);
        }
    }
}

impl Error for ParserError {
    fn description(&self) -> &str { "TOML parse error" }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.desc.fmt(f)
    }
}

fn is_digit(c: char) -> bool {
    match c { '0' ... '9' => true, _ => false }
}

#[cfg(test)]
mod tests {
    use Value::Table;
    use Parser;

    macro_rules! bad {
        ($s:expr, $msg:expr) => ({
            let mut p = Parser::new($s);
            assert!(p.parse().is_none());
            assert!(p.errors.iter().any(|e| e.desc.contains($msg)),
                    "errors: {:?}", p.errors);
        })
    }

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
                let mut p = Parser::new(&f);
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
        t!("2_0.0", 20.0);
        t!("2_0.0_0e0_0", 20.0);
        t!("2_0.1_0e1_0", 20.1e10);
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

    #[test]
    fn inline_tables() {
        assert!(Parser::new("a = {}").parse().is_some());
        assert!(Parser::new("a = {b=1}").parse().is_some());
        assert!(Parser::new("a = {   b   =   1    }").parse().is_some());
        assert!(Parser::new("a = {a=1,b=2}").parse().is_some());
        assert!(Parser::new("a = {a=1,b=2,c={}}").parse().is_some());
        assert!(Parser::new("a = {a=1,}").parse().is_none());
        assert!(Parser::new("a = {,}").parse().is_none());
        assert!(Parser::new("a = {a=1,a=1}").parse().is_none());
        assert!(Parser::new("a = {\n}").parse().is_none());
        assert!(Parser::new("a = {").parse().is_none());
        assert!(Parser::new("a = {a=[\n]}").parse().is_some());
        assert!(Parser::new("a = {\"a\"=[\n]}").parse().is_some());
        assert!(Parser::new("a = [\n{},\n{},\n]").parse().is_some());
    }

    #[test]
    fn number_underscores() {
        macro_rules! t {
            ($actual:expr, $expected:expr) => ({
                let f = format!("foo = {}", $actual);
                let mut p = Parser::new(&f);
                let table = Table(p.parse().unwrap());
                assert_eq!(table.lookup("foo").and_then(|k| k.as_integer()),
                           Some($expected));
            })
        }

        t!("1_0", 10);
        t!("1_0_0", 100);
        t!("1_000", 1000);
        t!("+1_000", 1000);
        t!("-1_000", -1000);
    }

    #[test]
    fn bad_underscores() {
        assert!(Parser::new("foo = 0_").parse().is_none());
        assert!(Parser::new("foo = 0__0").parse().is_none());
        assert!(Parser::new("foo = __0").parse().is_none());
        assert!(Parser::new("foo = 1_0_").parse().is_none());
    }

    #[test]
    fn bad_unicode_codepoint() {
        bad!("foo = \"\\uD800\"", "not a valid unicode codepoint");
    }

    #[test]
    fn bad_strings() {
        bad!("foo = \"\\uxx\"", "expected 4 hex digits");
        bad!("foo = \"\\u\"", "expected 4 hex digits");
        bad!("foo = \"\\", "unterminated");
        bad!("foo = '", "unterminated");
    }

    #[test]
    fn empty_string() {
        let mut p = Parser::new("foo = \"\"");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").unwrap().as_str(), Some(""));
    }

    #[test]
    fn booleans() {
        let mut p = Parser::new("foo = true");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").unwrap().as_bool(), Some(true));

        let mut p = Parser::new("foo = false");
        let table = Table(p.parse().unwrap());
        assert_eq!(table.lookup("foo").unwrap().as_bool(), Some(false));

        assert!(Parser::new("foo = true2").parse().is_none());
        assert!(Parser::new("foo = false2").parse().is_none());
        assert!(Parser::new("foo = t1").parse().is_none());
        assert!(Parser::new("foo = f2").parse().is_none());
    }

    #[test]
    fn bad_nesting() {
        bad!("
            a = [2]
            [[a]]
            b = 5
        ", "duplicate key `a` in table");
        bad!("
            a = 1
            [a.b]
        ", "key `a` was not previously a table");
        bad!("
            a = []
            [a.b]
        ", "array `a` does not contain tables");
        bad!("
            a = []
            [[a.b]]
        ", "array `a` does not contain tables");
        bad!("
            [a]
            b = { c = 2, d = {} }
            [a.b]
            c = 2
        ", "redefinition of table `b`");
    }

    #[test]
    fn bad_table_redefine() {
        bad!("
            [a]
            foo=\"bar\"
            [a.b]
            foo=\"bar\"
            [a]
        ", "redefinition of table `a`");
        bad!("
            [a]
            foo=\"bar\"
            b = { foo = \"bar\" }
            [a]
        ", "redefinition of table `a`");
        bad!("
            [a]
            b = {}
            [a.b]
        ", "redefinition of table `b`");

        bad!("
            [a]
            b = {}
            [a]
        ", "redefinition of table `a`");
    }
}
