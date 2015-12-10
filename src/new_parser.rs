use std::str;
use std::iter;
use std::borrow::Cow;

use self::Token::*;

pub enum Token<'a> {
    Whitespace(&'a str),
    Newline(&'a str),
    Comment(&'a str),

    Equals(&'a str),
    Period(&'a str),
    Comma(&'a str),
    LeftBrace(&'a str),
    RightBrace(&'a str),
    LeftBracket(&'a str),
    RightBracket(&'a str),
    LeftDoubleBracket(&'a str),
    RightDoubleBracket(&'a str),

    Keylike(&'a str),
    String { src: &'a str, val: Cow<'a, str> },
}

pub enum Error {
    BareCr(usize),
    InvalidCharInString(usize, char),
    NewlineInString(usize),
    UnterminatedString(usize),
    Unexpected(usize, char),
}

pub struct Tokenizer<'a> {
    input: &'a str,
    chars: iter::Peekable<str::CharIndices<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            input: input,
            chars: input.char_indices().peekable(),
        }
    }

    fn eat(&mut self, ch: char) -> bool {
        match self.chars.peek() {
            Some(&(_, ch2)) if ch == ch2 => {
                self.chars.next();
                true
            }
            _ => false,
        }
    }

    fn current(&mut self) -> usize {
        self.chars.peek().map(|i| i.0).unwrap_or(self.input.len())
    }

    fn crlf(&mut self, start: usize) -> Result<Token<'a>, Error> {
        if self.eat('\n') {
            Ok(Newline(&self.input[start..start+2]))
        } else {
            Err(Error::BareCr(start))
        }
    }

    fn whitespace(&mut self, start: usize) -> Token<'a> {
        while self.eat(' ') || self.eat('\t') {
            // ...
        }
        Whitespace(&self.input[start..self.current()])
    }

    fn comment(&mut self, start: usize) -> Token<'a> {
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch != '\t' && (ch < '\u{20}' || ch > '\u{10fff}') {
                break
            }
            self.chars.next();
        }
        Comment(&self.input[start..self.current()])
    }

    fn left_bracket(&mut self, start: usize) -> Token<'a> {
        if self.eat('[') {
            LeftDoubleBracket(&self.input[start..start+2])
        } else {
            LeftBracket(&self.input[start..start+1])
        }
    }

    fn right_bracket(&mut self, start: usize) -> Token<'a> {
        if self.eat('[') {
            RightDoubleBracket(&self.input[start..start+2])
        } else {
            RightBracket(&self.input[start..start+1])
        }
    }

    fn literal_string(&mut self, start: usize) -> Result<Token<'a>, Error> {
        let ok = |me: &Tokenizer<'a>, val_start, val_end, end| {
            Ok(String {
                src: &me.input[start..end],
                val: Cow::Borrowed(&me.input[val_start..val_end]),
            })
        };
        let mut multiline = false;
        if self.eat('\'') {
            if self.eat('\'') {
                multiline = true;
            } else {
                return ok(self, 0, 0, start + 2)
            }
        }
        let mut val_start = self.current();
        loop {
            match self.chars.next() {
                Some((end, '\'')) => {
                    if !multiline {
                        return ok(self, val_start, end, end + 1)
                    } else if self.eat('\'') && self.eat('\'') {
                        return ok(self, val_start, end, end + 3)
                    }
                }
                Some((i, c @ '\n')) |
                Some((i, c @ '\r')) => {
                    if multiline {
                        if c == '\r' {
                            try!(self.crlf(i));
                        }
                        if val_start == start + 3 {
                            val_start = self.current();
                        }
                    } else {
                        return Err(Error::NewlineInString(i))
                    }
                }
                Some((i, ch)) => {
                    if ch == '\u{09}' || ('\u{20}' <= ch && ch <= '\u{10fff}') {
                        continue
                    }
                    return Err(Error::InvalidCharInString(i, ch))
                }
                None => return Err(Error::UnterminatedString(start)),
            }
        }
    }

    fn basic_string(&mut self, start: usize) -> Result<Token<'a>, Error> {
        loop {}
        // let mut multiline = false;
        // let ok = |me: &Tokenizer<'a>, val_start, val_end, end| {
        //     Ok(String {
        //         src: &me.input[start..end],
        //         val: Cow::Borrowed(&me.input[val_start..val_end]),
        //     })
        // };
        // if self.eat('\'') {
        //     if self.eat('\'') {
        //         multiline = true;
        //     } else {
        //         return ok(self, 0, 0, start + 2)
        //     }
        // }
        // let mut val_start = self.current();
        // loop {
        //     match self.chars.next() {
        //         Some((end, '\'')) => {
        //             if !multiline {
        //                 return ok(self, val_start, end, end + 1)
        //             } else if self.eat('\'') && self.eat('\'') {
        //                 return ok(self, val_start, end, end + 3)
        //             }
        //         }
        //         Some((i, c @ '\n')) |
        //         Some((i, c @ '\r')) => {
        //             if multiline {
        //                 if c == '\r' {
        //                     try!(self.crlf(i));
        //                 }
        //                 if val_start == start + 3 {
        //                     val_start = self.current();
        //                 }
        //             } else {
        //                 return Err(Error::NewlineInLiteralString(i))
        //             }
        //         }
        //         Some((i, ch)) => {
        //             if ch == '\u{09}' || ('\u{20}' <= ch && ch <= '\u{10fff}') {
        //                 continue
        //             }
        //             return Err(Error::InvalidCharInLiteralString(i, ch))
        //         }
        //         None => return Err(Error::UnterminatedString(start)),
        //     }
        // }
    }

    fn keylike(&mut self, start: usize) -> Token<'a> {
        while let Some(&(_, ch)) = self.chars.peek() {
            if !is_keylike(ch) {
                break
            }
            self.chars.next();
        }
        Keylike(&self.input[start..self.current()])
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Result<Token<'a>, Error>> {
        let token = match self.chars.next() {
            Some((start, ' ')) => self.whitespace(start),
            Some((start, '\t')) => self.whitespace(start),
            Some((start, '\n')) => Newline(&self.input[start..start+1]),
            Some((start, '\r')) => return Some(self.crlf(start)),
            Some((start, '#')) => self.comment(start),
            Some((start, '=')) => Equals(&self.input[start..start+1]),
            Some((start, '.')) => Period(&self.input[start..start+1]),
            Some((start, ',')) => Comma(&self.input[start..start+1]),
            Some((start, '{')) => LeftBrace(&self.input[start..start+1]),
            Some((start, '}')) => RightBrace(&self.input[start..start+1]),
            Some((start, '[')) => self.left_bracket(start),
            Some((start, ']')) => self.right_bracket(start),
            Some((start, '\'')) => return Some(self.literal_string(start)),
            Some((start, '"')) => return Some(self.basic_string(start)),
            Some((start, ch)) if is_keylike(ch) => self.keylike(start),

            Some((i, ch)) => return Some(Err(Error::Unexpected(i, ch))),
            None => return None,
        };
        Some(Ok(token))
    }
}

fn is_keylike(ch: char) -> bool {
    ('A' <= ch && ch <= 'Z') ||
        ('a' <= ch && ch <= 'z') ||
        ('0' <= ch && ch <= '9') ||
        ch == '-' ||
        ch == '_'
}
