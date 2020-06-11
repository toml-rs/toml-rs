//! Tokenizer implementation for string literals.

use crate::{cursor::Cursor, Span};
use std::{char, iter};
/// Kind of TOML string literal.
///
/// Beware that even though this is currently `#[repr(u8)]`, its actual memory
/// layout is private and is not the target for semver compatibility.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum StrLitKind {
    /// Single-quoted strings that don't support escape sequences.
    Literal = b'\'',
    /// Double-quoted strings that do support escape sequences.
    Basic = b'"',
}

impl StrLitKind {
    /// Get the quote char of the string this kind represents
    pub const fn to_quote(self) -> char {
        self as u8 as char
    }
    /// Get the kind of the string from the specified quote char
    pub fn from_quote(quote: char) -> Option<StrLitKind> {
        match quote {
            '\'' => Some(StrLitKind::Literal),
            '"' => Some(StrLitKind::Basic),
            _ => None,
        }
    }
}

/// Represents the low-level subtokens of the string literal token.
/// The tokens represent both welformed and malformed string literals.
/// See the docs on the variants for more details.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StrLitSubtoken {
    /// Leading newline character in multiline strings (it should be skipped).
    LeadingNewline,
    /// Unicode escape sequence, represents a unicode scalar value of a character
    /// or the error of decoding one.
    UnicodeEscape {
        /// Type of the unicode escape (it is determined by `\u` or `\U` prefix)
        kind: HexLen,
        /// The result of decoding the unicode escape sequence.
        unescaped: UnicodeEscape,
    },
    /// Shorthand escape sequence for ad-hoc characters.
    /// These are one of: `\, \b, \f, \n, \r, \t, \", \'`,
    /// It will be `Ok(unescaped_char)` if the character after the slash does belong
    /// to the aformentioned set of shorthand escapes, otherwise the character
    /// that doesn't belong to that set will be stored as `Err(Some(unexpected_char))`
    /// Note that, there is a special case of Err(None), this denotes a trailing
    /// bare slash right at the end of unterminated string literal.
    ShorthandEscape(Result<char, Option<char>>),
    /// Exactly the character itself that doesn't need any escaping and can be used as is.
    Char(char),
    /// Character that cannot appear as a raw character in the string literal
    /// (e.g. raw control chars like `\0 \b`).
    /// Note: In order to represent them in TOML strings users have to utilize unicode escapes.
    BannedChar(char),
    /// Represents "line ending backslash" and the following trimmed whitespace characters.
    /// Example:
    /// ```toml
    /// key = """
    /// notice the slash, which trimms all the following whitespace -> \
    ///
    /// foo
    /// """
    /// ```
    TrimmedWhitespace {
        /// Defines whether there was a newline inside of the trimmed whitespace,
        /// if not, then the token is malformed
        includes_newline: bool,
    },
    /// Leading quotes, this the required first subtoken of the string literal.
    LeadingQuotes(Quotes),
    /// Optional trailing quotes (when not present the string literal is unterminated).
    TrailingQuotes,
}

/// Describes string literal quotes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Quotes {
    /// Length of quotes (`1` or `3`).
    pub len: QuotesLen,
    /// Kind of the string (`Basic` or `Literal`).
    pub kind: StrLitKind,
}

/// Represents the length of the quotes.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
pub enum QuotesLen {
    /// `'` or `"`
    X1 = 1,
    /// `'''` or `"""`
    X3 = 3,
}

impl From<QuotesLen> for usize {
    fn from(len: QuotesLen) -> usize {
        len as usize
    }
}

/// Result of tokenizing the unicode escape sequence of forms `\uXXXX` and `\UXXXXXXXX`
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum UnicodeEscape {
    /// Successfully parsed unicode character itself.
    Valid(char),
    /// Contains the actual amount of digits parsed, this is at most `7`.
    NotEnoughDigits(u32),
    /// Contains the scalar value itself. It cannot be turned into a valid
    /// unicode `char`, (i.e. into Rust `char` itself since Rust `char` **must** be
    /// a valid unicode character).
    InvalidScalarValue(u32),
}

/// Represents the number of hexadecimal digits in unicode escape sequence.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum HexLen {
    /// Unicode escape of form `\uXXXX`
    X4 = 4,
    /// Unicode escape of form `\UXXXXXXXX`
    X8 = 8,
}

impl From<HexLen> for u32 {
    fn from(len: HexLen) -> u32 {
        len as u32
    }
}

impl From<HexLen> for usize {
    fn from(len: HexLen) -> usize {
        len as usize
    }
}

/// The same as [`tokenize_str_lit()`], but consumes
/// the leading quotes and returns them as the first element of the tuple.
/// It is much more preffered to call this function if you wan't to check
/// the kind of the string literal without dealing with excessive `.unwrap()`
/// ```
/// # use toml_lexer::*;
///
/// // Good:
/// let (quotes_span, quotes, mut iter) = tokenize_str_lit_take_leading_quotes("'foo'")
///     .expect("The string clearly has leading quotes");
///
/// assert_eq!(iter.next(), Some((Span { start: 1, end: 2 }, StrLitSubtoken::Char('f'))));
///
/// // Bad:
/// let (_, token) = tokenize_str_lit("'foo'")
///     .expect("Yes, there are leading quotes")
///     .next()
///     .expect("There must be at least one leading quotes token!");
/// let quotes = match token {
///     StrLitSubtoken::LeadingQuotes(it) => it,
///     _ => unreachable!("Common, there first token is always the leading quotes!")
/// };
/// ```
///
/// [`tokenize_str_lit()`]: fn.tokenize_str_lit.html
pub fn tokenize_str_lit_take_leading_quotes(
    string_literal: &str,
) -> Option<(
    Span,
    Quotes,
    impl '_ + Iterator<Item = (Span, StrLitSubtoken)>,
)> {
    StrLitTokenizer::from_cursor(Cursor::new(string_literal))
}

// TODO: test quotes tokens
/// Tokenizes the leading string literal into its component subtokens.
/// It determines the kind of the literal by the leading quotes and if there are
/// no leading quotes, *only in this case* returns `None`.
///
/// ```
/// # use toml_lexer::*;
/// assert!(tokenize_str_lit("No leading quotes").is_none());
///
/// let tokens = tokenize_str_lit("\"<- leading quote!").unwrap();
/// let tokens: Vec<(Span, StrLitSubtoken)> = tokens.collect();
/// let quotes = Quotes {
///     kind: StrLitKind::Basic,
///     len: QuotesLen::X1
/// };
///
/// assert_eq!(tokens[0], (Span { start: 0, end: 1 }, StrLitSubtoken::LeadingQuotes(quotes)));
/// ```
///
pub fn tokenize_str_lit(
    string_literal: &str,
) -> Option<impl '_ + Iterator<Item = (Span, StrLitSubtoken)>> {
    let (quotes_span, quotes, tokenizer) = tokenize_str_lit_take_leading_quotes(string_literal)?;

    let first_token = StrLitSubtoken::LeadingQuotes(quotes);

    Some(iter::once((quotes_span, first_token)).chain(tokenizer))
}

/// String subtokenizer is a state machine since the tokens it expects
/// depend on the previous context.
#[derive(Clone)]
enum State {
    Begin,
    Content,
    End,
}

#[derive(Clone)]
pub(crate) struct StrLitTokenizer<'a> {
    cursor: Cursor<'a>,
    state: State,
    kind: StrLitKind,
    multiline: bool,
}

impl<'a> StrLitTokenizer<'a> {
    pub(crate) fn from_cursor(mut cursor: Cursor<'a>) -> Option<(Span, Quotes, Self)> {
        let start = cursor.current_index();
        let quote = cursor.one()?;
        let kind = StrLitKind::from_quote(quote)?;

        let len = if cursor.peek_two() == Some((quote, quote)) {
            cursor.one();
            cursor.one();
            QuotesLen::X3
        } else {
            QuotesLen::X1
        };

        let me = StrLitTokenizer {
            cursor,
            kind,
            state: State::Begin,
            multiline: len == QuotesLen::X3,
        };
        let leading_quotes = Quotes { kind, len };

        Some((me.cursor.span_from(start), leading_quotes, me))
    }

    pub(crate) fn cursor(&self) -> &Cursor<'a> {
        &self.cursor
    }

    fn unicode_hex(&mut self, len: HexLen) -> UnicodeEscape {
        debug_assert!(
            self.cursor.consumed_slice().ends_with("\\u")
                || self.cursor.consumed_slice().ends_with("\\U")
        );

        let mut code_point = 0u32;
        for n_digits in 0..len.into() {
            match self.cursor.peek_one().and_then(|ch| ch.to_digit(16)) {
                Some(digit) => code_point = (code_point * 16) + digit,
                _ => return UnicodeEscape::NotEnoughDigits(n_digits),
            }
            self.cursor.one();
        }
        std::char::from_u32(code_point)
            .map(UnicodeEscape::Valid)
            .unwrap_or_else(|| UnicodeEscape::InvalidScalarValue(code_point))
    }

    fn eat_3_trailing_quotes(&mut self) -> bool {
        let quote = self.kind.to_quote();

        debug_assert!(self.cursor.consumed_slice().ends_with(quote));

        let mut cloned = self.cursor.clone();
        // Lookahed 3 more chars to findout if the cursor is currently in the middle 3 closing quotes
        if !cloned.eatc(quote) || !cloned.eatc(quote) {
            return false;
        }
        if cloned.eatc(quote) {
            // this is the case of 4 consecutive quotes """"
            return false;
        }
        self.cursor.one();
        self.cursor.one();
        true
    }

    fn trimmed_whitespace(&mut self, begin: char) -> StrLitSubtoken {
        let mut includes_newline = begin == '\n';
        while let Some(ch @ ' ') | Some(ch @ '\t') | Some(ch @ '\n') = self.cursor.peek_one() {
            includes_newline = includes_newline || ch == '\n';
            self.cursor.one();
        }
        StrLitSubtoken::TrimmedWhitespace { includes_newline }
    }
}

impl Iterator for StrLitTokenizer<'_> {
    type Item = (Span, StrLitSubtoken);

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.cursor.current_index();

        let token = match self.state {
            State::Begin => {
                self.state = State::Content;
                if self.multiline && self.cursor.eatc('\n') {
                    StrLitSubtoken::LeadingNewline
                } else {
                    // single-stackframe recursive call with the new state
                    return self.next();
                }
            }
            State::Content => {
                let ch = self.cursor.peek_one()?;

                if ch == '\n' && !self.multiline {
                    return None;
                }

                self.cursor.one();

                match (self.kind, ch) {
                    (StrLitKind::Basic, '\\') => match self.cursor.one() {
                        None => StrLitSubtoken::ShorthandEscape(Err(None)),
                        Some(ch) => match ch {
                            '"' => StrLitSubtoken::ShorthandEscape(Ok('"')),
                            '\\' => StrLitSubtoken::ShorthandEscape(Ok('\\')),
                            'n' => StrLitSubtoken::ShorthandEscape(Ok('\n')),
                            'r' => StrLitSubtoken::ShorthandEscape(Ok('\r')),
                            't' => StrLitSubtoken::ShorthandEscape(Ok('\t')),
                            'b' => StrLitSubtoken::ShorthandEscape(Ok('\u{8}')),
                            'f' => StrLitSubtoken::ShorthandEscape(Ok('\u{c}')),
                            'u' | 'U' => {
                                let kind = if ch == 'u' { HexLen::X4 } else { HexLen::X8 };
                                let unescaped = self.unicode_hex(kind);
                                StrLitSubtoken::UnicodeEscape { unescaped, kind }
                            }
                            ' ' | '\t' | '\n' if self.multiline => self.trimmed_whitespace(ch),
                            _ => StrLitSubtoken::ShorthandEscape(Err(Some(ch))),
                        },
                    },
                    _ if ch == self.kind.to_quote() => {
                        if self.multiline && !self.eat_3_trailing_quotes() {
                            StrLitSubtoken::Char(ch)
                        } else {
                            self.state = State::End;
                            StrLitSubtoken::TrailingQuotes
                        }
                    }
                    _ if (ch >= '\u{20}' && ch != '\u{7f}') || matches!(ch, '\u{09}' | '\n') => {
                        StrLitSubtoken::Char(ch)
                    }
                    _ => StrLitSubtoken::BannedChar(ch),
                }
            }
            State::End => return None,
        };

        Some((self.cursor.span_from(start), token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_and_trailing_quotes() {
        let t = |input, kind, quotes_len: usize, trailing| {
            let it = tokenize_str_lit_take_leading_quotes(input).unwrap();
            let (_, actual_leading, tokens) = it;

            let leading = Quotes {
                kind,
                len: match quotes_len {
                    1 => QuotesLen::X1,
                    3 => QuotesLen::X3,
                    _ => unreachable!(),
                },
            };

            assert_eq!(leading, actual_leading);
            let last_token = tokens.last();
            let matches = matches!(last_token, Some((_, StrLitSubtoken::TrailingQuotes)));
            assert!(matches == trailing, "{:?}", last_token);
        };

        let t_none = |input| assert!(matches!(tokenize_str_lit_take_leading_quotes(input), None));

        t_none("");
        t_none("a");
        t_none("\\");

        t("'a", StrLitKind::Literal, 1, false);
        t("\"a", StrLitKind::Basic, 1, false);
        t("'''a", StrLitKind::Literal, 3, false);
        t("\"\"\"a", StrLitKind::Basic, 3, false);

        t("'a'", StrLitKind::Literal, 1, true);
        t("\"a\"", StrLitKind::Basic, 1, true);
        t("'''a'''", StrLitKind::Literal, 3, true);
        t("\"\"\"a\"\"\"", StrLitKind::Basic, 3, true);

        t("''''", StrLitKind::Literal, 3, false);
        t("\"\"\"\"", StrLitKind::Basic, 3, false);

        t("'''''", StrLitKind::Literal, 3, false);
        t("\"\"\"\"\"", StrLitKind::Basic, 3, false);

        t("'", StrLitKind::Literal, 1, false);
        t("\"", StrLitKind::Basic, 1, false);
        t("''", StrLitKind::Literal, 1, true);
        t("\"\"", StrLitKind::Basic, 1, true);

        t("'''", StrLitKind::Literal, 3, false);
        t("\"\"\"", StrLitKind::Basic, 3, false);
        t("''''''", StrLitKind::Literal, 3, true);
        t("\"\"\"\"\"\"", StrLitKind::Basic, 3, true);
    }

    mod unicode_escapes {
        use super::*;

        fn assert_unicode_escape(len: HexLen, input: &str, expected: UnicodeEscape) {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::UnicodeEscape {
                    kind: len,
                    unescaped: expected,
                },
                &["\"", "\"\"\""],
            );
        }

        #[test]
        fn valid_unicode_escapes() {
            let t = |len, input, expected| {
                assert_unicode_escape(len, input, UnicodeEscape::Valid(expected))
            };

            t(HexLen::X4, "\\u0000", '\u{0}');
            t(HexLen::X4, "\\u1f1A", '\u{1F1A}');
            t(HexLen::X4, "\\u0019", '\u{19}');

            t(HexLen::X8, "\\U00000000", '\u{0}');
            t(HexLen::X8, "\\U000AbBcD", '\u{ABBCD}');
            t(HexLen::X8, "\\U0010FFFF", '\u{10FFFF}');

            let valid_unicode_escape = |kind, unescaped| StrLitSubtoken::UnicodeEscape {
                kind,
                unescaped: UnicodeEscape::Valid(unescaped),
            };
            assert_string_content_subtokens(
                "\\u12345\\U000456789F",
                &["\"", "\"\"\""],
                vec![
                    ((0, 6), valid_unicode_escape(HexLen::X4, '\u{1234}')),
                    ((6, 7), StrLitSubtoken::Char('5')),
                    ((7, 17), valid_unicode_escape(HexLen::X8, '\u{45678}')),
                    ((17, 18), StrLitSubtoken::Char('9')),
                    ((18, 19), StrLitSubtoken::Char('F')),
                ],
            );
        }

        #[test]
        fn not_enough_digits_in_unicode_escapes() {
            let t = |len, input, expected| {
                assert_unicode_escape(len, input, UnicodeEscape::NotEnoughDigits(expected))
            };

            t(HexLen::X4, "\\u", 0);
            t(HexLen::X4, "\\u0", 1);
            t(HexLen::X4, "\\u00", 2);
            t(HexLen::X4, "\\u000", 3);
            t(HexLen::X8, "\\U", 0);
            t(HexLen::X8, "\\U0", 1);
            t(HexLen::X8, "\\U00", 2);
            t(HexLen::X8, "\\U000", 3);
            t(HexLen::X8, "\\U0000", 4);
            t(HexLen::X8, "\\U00000", 5);
            t(HexLen::X8, "\\U000000", 6);
            t(HexLen::X8, "\\U0000000", 7);
        }

        #[test]
        fn invalid_scalar_value() {
            let t = |len, input, expected| {
                assert_unicode_escape(len, input, UnicodeEscape::InvalidScalarValue(expected))
            };
            t(HexLen::X4, "\\uD800", 0xd800);
            t(HexLen::X8, "\\U00110000", 0x0011_0000);
            t(HexLen::X8, "\\Uffffffff", 0xffff_ffff);
        }
    }

    mod shorthand_escapes {
        use super::*;

        fn t_all(input: &str, result: Result<char, Option<char>>) {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::ShorthandEscape(result),
                &["\"", "\"\"\""],
            );
        }

        fn t_single_line(input: &str, result: Result<char, Option<char>>) {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::ShorthandEscape(result),
                &["\""],
            );
        }

        #[test]
        fn valid_shorthand_escapes() {
            t_all(r#"\b"#, Ok('\u{0008}'));
            t_all(r#"\t"#, Ok('\u{0009}'));
            t_all(r#"\n"#, Ok('\u{000A}'));
            t_all(r#"\f"#, Ok('\u{000C}'));
            t_all(r#"\r"#, Ok('\u{000D}'));
            t_all(r#"\""#, Ok('\u{0022}'));
            t_all(r#"\\"#, Ok('\u{005C}'));
        }

        #[test]
        fn invalid_shorthand_escapes() {
            t_all(r#"\a"#, Err(Some('a')));
            t_all("\\\u{0}", Err(Some('\u{0}')));
            t_all("\\🦀", Err(Some('🦀')));

            t_single_line("\\\r\n", Err(Some('\n')));
            t_single_line("\\\n", Err(Some('\n')));
        }

        #[test]
        fn trailing_slash() {
            // May appear only in unterminated basic strings
            let acutal: Vec<_> = tokenize_str_lit("\"\\").unwrap().collect();
            assert_eq!(
                acutal,
                vec![
                    (
                        Span { start: 0, end: 1 },
                        StrLitSubtoken::LeadingQuotes(Quotes {
                            kind: StrLitKind::Basic,
                            len: QuotesLen::X1
                        })
                    ),
                    (
                        Span { start: 1, end: 2 },
                        StrLitSubtoken::ShorthandEscape(Err(None))
                    )
                ]
            );

            let acutal: Vec<_> = tokenize_str_lit("\"\"\"\\").unwrap().collect();
            assert_eq!(
                acutal,
                vec![
                    (
                        Span { start: 0, end: 3 },
                        StrLitSubtoken::LeadingQuotes(Quotes {
                            kind: StrLitKind::Basic,
                            len: QuotesLen::X3
                        })
                    ),
                    (
                        Span { start: 3, end: 4 },
                        StrLitSubtoken::ShorthandEscape(Err(None))
                    )
                ]
            );
        }
    }

    #[test]
    fn banned_chars() {
        let t = |input, expected| {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::BannedChar(expected),
                &["\"", "\"\"\"", "'", "'''"],
            )
        };

        t("\u{0}", '\u{0}');
        t("\u{1}", '\u{1}');
        t("\u{18}", '\u{18}');
        t("\u{19}", '\u{19}');
        t("\u{7f}", '\u{7f}');
    }

    #[test]
    fn leading_newline() {
        let t = |input| {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::LeadingNewline,
                &["\"\"\"", "'''"],
            )
        };
        t("\n");
        t("\r\n");
    }

    #[test]
    fn valid_char_itself() {
        let t = |input, expected| {
            assert_single_string_content_subtoken(
                input,
                StrLitSubtoken::Char(expected),
                &["\"", "\"\"\"", "'", "'''"],
            );
        };

        t("a", 'a');
        t("Ї", 'Ї');
        t("🦀", '🦀');
        t("©", '©');
    }

    #[test]
    fn escapes_are_ignored_in_literal_strings() {
        assert_string_content_subtokens(
            "\\u1234\\n\\ \t ",
            &["'", "'''"],
            vec![
                ((0, 1), StrLitSubtoken::Char('\\')),
                ((1, 2), StrLitSubtoken::Char('u')),
                ((2, 3), StrLitSubtoken::Char('1')),
                ((3, 4), StrLitSubtoken::Char('2')),
                ((4, 5), StrLitSubtoken::Char('3')),
                ((5, 6), StrLitSubtoken::Char('4')),
                ((6, 7), StrLitSubtoken::Char('\\')),
                ((7, 8), StrLitSubtoken::Char('n')),
                ((8, 9), StrLitSubtoken::Char('\\')),
                ((9, 10), StrLitSubtoken::Char(' ')),
                ((10, 11), StrLitSubtoken::Char('\t')),
                ((11, 12), StrLitSubtoken::Char(' ')),
            ],
        );
    }

    #[test]
    fn valid_trimmed_whitespace() {
        use assert_string_content_subtokens as t;

        t(
            "\\ \n  ",
            &["\"\"\""],
            vec![(
                (0, 5),
                StrLitSubtoken::TrimmedWhitespace {
                    includes_newline: true,
                },
            )],
        );
        t(
            " \\ \n \t:\t",
            &["\"\"\""],
            vec![
                ((0, 1), StrLitSubtoken::Char(' ')),
                (
                    (1, 6),
                    StrLitSubtoken::TrimmedWhitespace {
                        includes_newline: true,
                    },
                ),
                ((6, 7), StrLitSubtoken::Char(':')),
                ((7, 8), StrLitSubtoken::Char('\t')),
            ],
        );
    }

    #[test]
    fn trimmed_whitespace_with_no_newline() {
        use assert_string_content_subtokens as t;

        t(
            "\\   ",
            &["\"\"\""],
            vec![(
                (0, 4),
                StrLitSubtoken::TrimmedWhitespace {
                    includes_newline: false,
                },
            )],
        );
        t(
            " \\  \t:\t",
            &["\"\"\""],
            vec![
                ((0, 1), StrLitSubtoken::Char(' ')),
                (
                    (1, 5),
                    StrLitSubtoken::TrimmedWhitespace {
                        includes_newline: false,
                    },
                ),
                ((5, 6), StrLitSubtoken::Char(':')),
                ((6, 7), StrLitSubtoken::Char('\t')),
            ],
        );
    }

    fn quotes_permutations(contents: &str, quotes_cases: &[&str]) -> Vec<String> {
        quotes_cases
            .iter()
            .flat_map(|quotes| {
                let terminated = format!("{}{}{}", quotes, contents, quotes);
                let unterminated = format!("{}{}", quotes, contents);
                iter::once(terminated).chain(iter::once(unterminated))
            })
            .collect()
    }

    fn assert_string_content_subtokens(
        input_contents: &str,
        quotes_cases: &[&str],
        expected: Vec<((usize, usize), StrLitSubtoken)>,
    ) {
        assert!(quotes_cases
            .iter()
            .all(|&case| matches!(case, "'" | "\"" | "'''" | "\"\"\"")));

        for string_literal in quotes_permutations(input_contents, quotes_cases) {
            let it = tokenize_str_lit_take_leading_quotes(&string_literal).unwrap();
            let (_, leading_quotes, actual) = it;

            let quotes_len: usize = leading_quotes.len.into();

            let actual: Vec<_> = actual
                .filter(|(_, token)| *token != StrLitSubtoken::TrailingQuotes)
                .map(|(span, token)| ((span.start - quotes_len, span.end - quotes_len), token))
                .collect();

            assert_eq!(actual, expected, "\nstring_literal: {{{}}}", string_literal);
        }
    }

    fn assert_single_string_content_subtoken(
        contents: &str,
        token: StrLitSubtoken,
        quotes_cases: &[&str],
    ) {
        assert_string_content_subtokens(contents, quotes_cases, vec![((0, contents.len()), token)]);
    }
}
