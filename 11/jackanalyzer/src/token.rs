use std::iter::Peekable;

#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    Keyword(&'a str),
    Symbol(char),
    Identifier(&'a str),
    IntegerConstant(i32),
    StringConstant(&'a str),
}

impl<'a> Token<'a> {
    pub(crate) fn unwrap_identifier(&self) -> &'a str {
        match self {
            Token::Identifier(ident) => ident,
            v => panic!("Expected identifier, got {v:?}"),
        }
    }
    pub(crate) fn unwrap_keyword(&self) -> &'a str {
        match self {
            Token::Keyword(kw) => kw,
            v => panic!("Expected keyword, got {v:?}"),
        }
    }

    pub fn unwrap_symbol(&self) -> char {
        match self {
            Token::Symbol(s) => *s,
            v => panic!("Expected symbol, got {v:?}"),
        }
    }
}

impl<'a> Token<'a> {
    pub fn parse_next(s: &'a str) -> Option<(Self, &str)> {
        let first_char = s.chars().next()?;
        Some(if let Some((kw, new_rest)) = keyword_token(s) {
            (Token::Keyword(kw), new_rest)
        } else if let Some(new_rest) = s.strip_prefix([
            '{', '}', '(', ')', '[', ']', '.', ',', ';', '+', '-', '*', '/', '&', '|', '<', '>',
            '=', '~',
        ]) {
            (Token::Symbol(first_char), new_rest)
        } else if let Some(new_rest) = s.strip_prefix('"') {
            let (string_const, new_rest) = new_rest
                .split_once('"')
                .expect("String value not misses trailing `\"`");
            (Token::StringConstant(string_const), new_rest)
        } else if first_char.is_ascii_digit() {
            let idx = s
                .find(|c: char| !c.is_ascii_digit())
                .expect("Trailing digit");
            (Token::IntegerConstant(s[..idx].parse().unwrap()), &s[idx..])
        } else {
            let idx = s
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .expect("Unterminated identifier");
            (Token::Identifier(&s[..idx]), &s[idx..])
        })
    }
}

fn strip_prefix_and_rest<'a, 'b>(s: &'a str, prefix: &'b str) -> Option<(&'b str, &'a str)> {
    s.strip_prefix(prefix).map(|rest| (prefix, rest))
}

fn keyword_token(s: &str) -> Option<(&str, &str)> {
    for keyword in [
        "class",
        "constructor",
        "function",
        "method",
        "field",
        "static",
        "var",
        "int",
        "char",
        "boolean",
        "void",
        "true",
        "false",
        "null",
        "this",
        "let",
        "do",
        "if",
        "else",
        "while",
        "return",
    ] {
        if let Some((keyword, rest)) = strip_prefix_and_rest(s, keyword) {
            if !rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
                return Some((keyword, rest));
            }
        }
    }
    None
}

pub struct TokenStream<'a> {
    inner: Peekable<InnerTokenStream<'a>>,
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = Token<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> TokenStream<'a> {
    pub fn new(s: &str) -> TokenStream {
        let inner = InnerTokenStream { rest: s }.peekable();
        TokenStream { inner }
    }

    pub fn peek(&mut self) -> Option<&Token<'_>> {
        self.inner.peek()
    }

    pub(crate) fn unwrap_symbol(&mut self) -> char {
        self.next().unwrap().unwrap_symbol()
    }

    pub(crate) fn unwrap_keyword(&mut self) -> &'a str {
        self.next().unwrap().unwrap_keyword()
    }

    pub(crate) fn unwrap_identifier(&mut self) -> &'a str {
        self.next().unwrap().unwrap_identifier()
    }
}

struct InnerTokenStream<'a> {
    rest: &'a str,
}

impl<'a> Iterator for InnerTokenStream<'a> {
    type Item = Token<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let result;
        (result, self.rest) = Token::parse_next(self.rest.trim())?;
        Some(result)
    }
}
