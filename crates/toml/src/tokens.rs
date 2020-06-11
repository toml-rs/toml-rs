use std::{borrow::Cow, str};
pub(crate) use toml_lexer::{HexLen, QuotesLen, Span};

// Entirely wellformed token of TOML language
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Token<'s> {
    Whitespace(&'s str),
    Newline,
    Comment(&'s str),

    Equals,
    Period,
    Comma,
    Colon,
    Plus,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,

    Keylike(&'s str),
    String {
        src: &'s str,
        val: Cow<'s, str>,
        multiline: bool,
    },
}

impl Token<'_> {
    pub(crate) fn describe(&self) -> &'static str {
        match *self {
            Token::Keylike(_) => "an identifier",
            Token::Equals => "an equals",
            Token::Period => "a period",
            Token::Comment(_) => "a comment",
            Token::Newline => "a newline",
            Token::Whitespace(_) => "whitespace",
            Token::Comma => "a comma",
            Token::RightBrace => "a right brace",
            Token::LeftBrace => "a left brace",
            Token::RightBracket => "a right bracket",
            Token::LeftBracket => "a left bracket",
            Token::String { multiline, .. } => {
                if multiline {
                    "a multiline string"
                } else {
                    "a string"
                }
            }
            Token::Colon => "a colon",
            Token::Plus => "a plus",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Error {
    InvalidCharInString(usize, char),
    InvalidShorthandEscape(usize, Option<char>),
    NotEnoughDigitsInHex {
        at: usize,
        expected: HexLen,
        actual: u32,
    },
    InvalidEscapeValue(usize, u32),
    UnterminatedString(usize),
    NoNewlineInTrimmedWhitespace(usize),
    Unexpected(usize, char),
    NewlineInTableKey(usize),
    MultilineStringKey(usize),
    EmptyTableKey(usize),
    Wanted {
        at: usize,
        expected: &'static str,
        found: &'static str,
    },
}

/// Error-reluctant tokenizer. It bails upon the first error detected
/// in the input TOML source string. We build it on top of error-resilient
/// one which is very generic on purpose and resides in `toml_lexer` crate.
#[derive(Clone)]
pub(crate) struct Tokenizer<'s> {
    inner: toml_lexer::Tokenizer<'s>,
}

impl<'s> Tokenizer<'s> {
    pub(crate) fn new(input: &'s str) -> Tokenizer<'s> {
        Tokenizer {
            inner: toml_lexer::Tokenizer::new(input),
        }
    }

    pub(crate) fn input(&self) -> &'s str {
        self.inner.input()
    }

    pub(crate) fn peek(&self) -> Result<Option<(Span, Token<'s>)>, Error> {
        self.clone().next()
    }

    pub fn current_index(&self) -> usize {
        self.inner.current_index()
    }

    fn input_slice(&self, span: Span) -> &'s str {
        &self.inner.input()[span.start..span.end]
    }

    pub(crate) fn next(&mut self) -> Result<Option<(Span, Token<'s>)>, Error> {
        let token = match self.inner.next() {
            Some(it) => it,
            None => return Ok(None),
        };

        let (leading_quotes_span, leading_quotes) = match self.into_non_str_lit(token) {
            Ok(non_str_lit) => return non_str_lit,
            Err(it) => it,
        };

        let quotes = match leading_quotes {
            toml_lexer::StrLitSubtoken::LeadingQuotes(it) => it,
            _ => unreachable!("String always begins with leading quotes!"),
        };

        self.unescape_next_str_lit(leading_quotes_span, quotes)
    }

    fn into_non_str_lit(
        &self,
        token: toml_lexer::Token,
    ) -> Result<Result<Option<(Span, Token<'s>)>, Error>, (Span, toml_lexer::StrLitSubtoken)> {
        let wellformed_token = match token.kind {
            toml_lexer::TokenKind::Whitespace => Token::Whitespace(self.input_slice(token.span)),
            toml_lexer::TokenKind::Newline => Token::Newline,
            toml_lexer::TokenKind::Comment => Token::Comment(self.input_slice(token.span)),
            toml_lexer::TokenKind::Equals => Token::Equals,
            toml_lexer::TokenKind::Period => Token::Period,
            toml_lexer::TokenKind::Comma => Token::Comma,
            toml_lexer::TokenKind::Colon => Token::Colon,
            toml_lexer::TokenKind::Plus => Token::Plus,
            toml_lexer::TokenKind::LeftBrace => Token::LeftBrace,
            toml_lexer::TokenKind::RightBrace => Token::RightBrace,
            toml_lexer::TokenKind::LeftBracket => Token::LeftBracket,
            toml_lexer::TokenKind::RightBracket => Token::RightBracket,
            toml_lexer::TokenKind::Keylike => Token::Keylike(self.input_slice(token.span)),
            toml_lexer::TokenKind::Unknown => {
                let slice = self.input_slice(token.span);
                return Ok(Err(Error::Unexpected(
                    token.span.start,
                    slice.chars().next().unwrap(),
                )));
            }
            toml_lexer::TokenKind::StrLitSubtoken(it) => return Err((token.span, it)),
        };

        Ok(Ok(Some((token.span, wellformed_token))))
    }

    fn unescape_next_str_lit(
        &mut self,
        leading_quotes_span: Span,
        quotes: toml_lexer::Quotes,
    ) -> Result<Option<(Span, Token<'s>)>, Error> {
        let mut unescaped = MaybeEscaped::new(self.input(), leading_quotes_span.end);

        let end = loop {
            let (span, subtoken) = match self.inner.clone().next() {
                Some(toml_lexer::Token {
                    span,
                    kind: toml_lexer::TokenKind::StrLitSubtoken(it),
                }) => (span, it),
                _ => return Err(Error::UnterminatedString(leading_quotes_span.start)),
            };
            self.inner.next();
            unescaped.append(match subtoken {
                toml_lexer::StrLitSubtoken::UnicodeEscape { unescaped, kind } => match unescaped {
                    toml_lexer::UnicodeEscape::Valid(ch) => ch,
                    toml_lexer::UnicodeEscape::NotEnoughDigits(actual) => {
                        return Err(Error::NotEnoughDigitsInHex {
                            at: span.start,
                            actual,
                            expected: kind,
                        })
                    }
                    toml_lexer::UnicodeEscape::InvalidScalarValue(val) => {
                        return Err(Error::InvalidEscapeValue(span.start + 1, val))
                    }
                },
                toml_lexer::StrLitSubtoken::ShorthandEscape(ch) => match ch {
                    Ok(ch) => ch,
                    Err(ch) => return Err(Error::InvalidShorthandEscape(span.start + 1, ch)),
                },
                toml_lexer::StrLitSubtoken::Char(ch) => ch,
                toml_lexer::StrLitSubtoken::BannedChar(it) => {
                    return Err(Error::InvalidCharInString(span.start, it))
                }
                toml_lexer::StrLitSubtoken::TrimmedWhitespace {
                    includes_newline: false,
                } => return Err(Error::NoNewlineInTrimmedWhitespace(span.start)),
                toml_lexer::StrLitSubtoken::TrimmedWhitespace {
                    includes_newline: true,
                }
                | toml_lexer::StrLitSubtoken::LeadingQuotes { .. }
                | toml_lexer::StrLitSubtoken::LeadingNewline => {
                    unescaped.skip(span);
                    continue;
                }
                toml_lexer::StrLitSubtoken::TrailingQuotes => break span.end,
            });
        };

        let span = Span {
            start: leading_quotes_span.start,
            end,
        };

        Ok(Some((
            span,
            Token::String {
                src: self.input_slice(span),
                val: unescaped.into_cow(),
                multiline: quotes.len == QuotesLen::X3,
            },
        )))
    }

    pub(crate) fn eat(&mut self, expected: Token<'s>) -> Result<bool, Error> {
        self.eat_spanned(expected).map(|s| s.is_some())
    }

    /// Eat a value, returning it's span if it was consumed.
    pub(crate) fn eat_spanned(&mut self, expected: Token<'s>) -> Result<Option<Span>, Error> {
        let span = match self.peek()? {
            Some((span, ref found)) if expected == *found => span,
            Some(_) => return Ok(None),
            None => return Ok(None),
        };

        drop(self.next());
        Ok(Some(span))
    }

    pub(crate) fn expect(&mut self, expected: Token<'s>) -> Result<(), Error> {
        // ignore span
        let _ = self.expect_spanned(expected)?;
        Ok(())
    }

    /// Expect the given token returning its span.
    pub(crate) fn expect_spanned(&mut self, expected: Token<'s>) -> Result<Span, Error> {
        let current = self.current_index();
        match self.next()? {
            Some((span, found)) => {
                if expected == found {
                    Ok(span)
                } else {
                    Err(Error::Wanted {
                        at: current,
                        expected: expected.describe(),
                        found: found.describe(),
                    })
                }
            }
            None => Err(Error::Wanted {
                at: self.input().len(),
                expected: expected.describe(),
                found: "eof",
            }),
        }
    }

    pub(crate) fn table_key(&mut self) -> Result<(Span, Cow<'s, str>), Error> {
        let current = self.current_index();
        match self.next()? {
            Some((span, Token::Keylike(k))) => Ok((span, k.into())),
            Some((
                span,
                Token::String {
                    src,
                    val,
                    multiline,
                },
            )) => {
                let offset = self.substr_offset(src);
                if multiline {
                    return Err(Error::MultilineStringKey(offset));
                }
                if val == "" {
                    return Err(Error::EmptyTableKey(offset));
                }
                match src.find('\n') {
                    None => Ok((span, val)),
                    Some(i) => Err(Error::NewlineInTableKey(offset + i)),
                }
            }
            Some((_, other)) => Err(Error::Wanted {
                at: current,
                expected: "a table key",
                found: other.describe(),
            }),
            None => Err(Error::Wanted {
                at: self.input().len(),
                expected: "a table key",
                found: "eof",
            }),
        }
    }

    pub(crate) fn eat_whitespace(&mut self) -> Result<(), Error> {
        while let Ok(Some((_, Token::Whitespace(_)))) = self.peek() {
            drop(self.next());
        }
        // TODO: rethink whether this method should return Result<>
        Ok(())
    }

    pub(crate) fn eat_comment(&mut self) -> Result<bool, Error> {
        if !matches!(self.peek()?, Some((_, Token::Comment(_)))) {
            return Ok(false);
        }
        drop(self.next());
        self.eat_newline_or_eof().map(|()| true)
    }

    pub(crate) fn eat_newline_or_eof(&mut self) -> Result<(), Error> {
        let current = self.current_index();
        match self.next()? {
            None | Some((_, Token::Newline)) => Ok(()),
            Some((_, other)) => Err(Error::Wanted {
                at: current,
                expected: "newline",
                found: other.describe(),
            }),
        }
    }

    pub(crate) fn skip_to_newline(&mut self) {
        while !matches!(self.peek(), Err(_) | Ok(None) | Ok(Some((_, Token::Newline)))) {
            drop(self.next());
        }
    }

    pub(crate) fn substr_offset(&self, s: &'s str) -> usize {
        assert!(s.len() <= self.input().len());
        let a = self.input().as_ptr() as usize;
        let b = s.as_ptr() as usize;
        assert!(a <= b);
        b - a
    }
}

#[derive(Debug)]
enum MaybeEscaped<'s> {
    NotEscaped(&'s str, str::CharIndices<'s>, usize),
    Escaped(String),
}

// TODO: test
impl<'s> MaybeEscaped<'s> {
    fn new(source: &str, begin: usize) -> MaybeEscaped<'_> {
        MaybeEscaped::NotEscaped(source, source[begin..].char_indices(), begin)
    }

    fn skip(&mut self, span: Span) {
        match self {
            // Move the start of the slice further if we are skipping
            // right from the beginning of the string
            MaybeEscaped::NotEscaped(source, char_indices, begin) if span.start == *begin => {
                *begin = span.end;
                *char_indices = source[*begin..].char_indices();
            }
            MaybeEscaped::NotEscaped(..) => {
                // Don't do anything since we might be skipping right until the end of the string.
                // If not, the next `append()` will upgrade us to owned variant
            }
            MaybeEscaped::Escaped(_) => {}
        }
    }

    fn append(&mut self, ch: char) {
        match self {
            MaybeEscaped::NotEscaped(source, char_indices, begin) => {
                let (i, contents_char) = char_indices
                    .next()
                    .expect("the caller (i.e. tokenizer) has eaten at least one char `ch`");
                if ch == contents_char {
                    // .. we may reuse the source string
                } else {
                    *self = MaybeEscaped::Escaped(format!("{}{}", &source[*begin..*begin + i], ch));
                }
            }
            MaybeEscaped::Escaped(it) => it.push(ch),
        }
    }

    fn into_cow(self) -> Cow<'s, str> {
        match self {
            MaybeEscaped::NotEscaped(source, mut char_indices, begin) => {
                match char_indices.next() {
                    Some((end, _)) => Cow::Borrowed(&source[begin..begin + end]),
                    None => Cow::Borrowed(&source[begin..]),
                }
            }
            MaybeEscaped::Escaped(it) => Cow::Owned(it),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_matches {
        ($val:expr, $pat:pat) => {{
            let val = &$val;
            if !matches!(val, $pat) {
                let val_str = stringify!($val);
                let pat_str = stringify!($pat);
                panic!(
                    "{} doesn't match the pattern: {}\n actual value: {:?}",
                    val_str, pat_str, val
                );
            }
        }};
    }

    fn unescape(literal: &str) -> Cow<'_, str> {
        match Tokenizer::new(literal).next().unwrap().unwrap().1 {
            Token::String { val, .. } => val,
            it => panic!("Lexed not a string-literal: {:?}", it),
        }
    }

    #[test]
    fn unescape_borrows_when_possible() {
        assert_matches!(unescape("'abcdef'"), Cow::Borrowed("abcdef"));
        assert_matches!(unescape("'abcdef'"), Cow::Borrowed("abcdef"));
        assert_matches!(unescape("'''abcdef''''"), Cow::Borrowed("abcdef'"));
        assert_matches!(unescape("'''\nabcdef'''"), Cow::Borrowed("abcdef"));
        assert_matches!(
            unescape("\"\"\"\\ \t \n \t abcdef\"\"\""),
            Cow::Borrowed("abcdef")
        );
        assert_matches!(unescape(r#""\\""#), Cow::Borrowed("\\"));
    }

    #[test]
    fn unescape_returns_owned_when_meets_escapes() {
        use unescape;

        let t = |input, expected: &str| {
            let actual = unescape(input);
            assert_matches!(actual, Cow::Owned(_));
            assert_eq!(actual, Cow::Borrowed(expected));
        };

        t(r#""\t""#, "\t");
        t("\"\"\"abc\\ \n a\"\"\"", "abca");
        t(r#""\u1234""#, "\u{1234}");
    }

    mod strings {
        use super::*;

        #[test]
        fn terminated_empty_strings() {
            use assert_empty_string as t;

            t("''", false);
            t("''''''", true);
            t("'''\n'''", true);

            t(r#""""#, false);
            t(r#""""""""#, true);

            t("\"\"\"\n\"\"\"", true);
            t("\"\"\"\n\\\n  \t\t  \"\"\"", true);
        }

        #[test]
        fn single_char_strings() {
            use assert_single_string as t;

            t("'a'", "a", false);
            t("' '", " ", false);
            t("'\t'", "\t", false);
            t("'''a'''", "a", true);
            t("''' '''", " ", true);
            t("'''\t'''", "\t", true);
            t("'''''''", "'", true);
            t(r#""a""#, "a", false);
            t(r#""\t""#, "\t", false);
            t("\"\t\"", "\t", false);
            t(r#""""a""""#, "a", true);
            t(r#"""""""""#, "\"", true);
            t(r#""""\t""""#, "\t", true);
            t("\"\"\"\t\"\"\"", "\t", true);
            t(r#""""\"""""#, "\"", true);
        }

        #[test]
        fn multi_char_strings() {
            use assert_single_string as t;

            t("'ab'", "ab", false);
            t("'\"a'", "\"a", false);
            t("'\\t'", "\\t", false);

            t("''''''''", "''", true);
            t(r#""""""""""#, "\"\"", true);

            t("'''\n'a\n'''", "'a\n", true);
            t("'''a\n'a\r\n'''", "a\n'a\n", true);
            t("'\\U00'", "\\U00", false);

            t(r#""\\t""#, "\\t", false);
            t(r#""""\\t""""#, "\\t", true);
        }

        #[test]
        fn unterminated_strings() {
            let t = |input| err(input, Error::UnterminatedString(0));

            t("'''''");
            t(r#"""""""#);
            t("''''");
            t(r#""""""#);

            t("'a");
            t("'\\");

            t(r#""a"#);
            t(r#""\""#);

            t("'''a");
            t("'''\\");

            t(r#""""a"#);
            t(r#""""\""#);
        }

        #[test]
        fn with_escapes() {
            use assert_single_string as t;

            t("\"\"\"\n\t\"\"\"", "\t", true);
            t("\"\"\"\\\n\"\"\"", "", true);
            t(
                "\"\"\"\\\n     \t   \t  \\\r\n  \t \n  \t \r\n\"\"\"",
                "",
                true,
            );
            t(r#""\r""#, "\r", false);
            t(r#""\n""#, "\n", false);
            t(r#""\b""#, "\u{8}", false);
            t(r#""a\fa""#, "a\u{c}a", false);
            t(r#""\"a""#, "\"a", false);
            t("\"\"\"\na\"\"\"", "a", true);
            t(r#""""a\"""b""""#, "a\"\"\"b", true);
        }

        fn assert_single_string(input: &str, expected_unescaped: &str, multiline: bool) {
            assert_single_token(
                input,
                Token::String {
                    src: input,
                    val: Cow::Borrowed(expected_unescaped),
                    multiline,
                },
            );
            assert_eq!(unescape(input), expected_unescaped, "input: {{{}}}", input);
        }

        fn assert_empty_string(input: &str, multiline: bool) {
            assert_single_string(input, "", multiline);
        }
    }

    fn assert_tokens(input: &str, expected: Vec<((usize, usize), Token<'_>, &str)>) {
        let mut sut = Tokenizer::new(input);
        let mut actual: Vec<_> = Vec::new();
        while let Some((span, token)) = sut.next().unwrap() {
            actual.push((span.into(), token, &input[span.start..span.end]));
        }
        assert_eq!(actual, expected, "input: {}", input);
    }

    fn assert_single_token(input: &str, expected: Token<'_>) {
        assert_tokens(input, vec![((0, input.len()), expected, input)]);
    }

    fn err(input: &str, err: Error) {
        let mut t = Tokenizer::new(input);
        let token = t.next().unwrap_err();
        assert_eq!(token, err);
        assert!(t.next().unwrap().is_none());
    }
}
