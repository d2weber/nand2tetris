use std::iter::Peekable;

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Keyword(&'a str),
    Symbol(char),
    Identifier(&'a str),
    IntegerConstant(i32),
    StringConstant(&'a str),
}

impl<'a> Token<'a> {
    pub(crate) fn write_xml(&self, out: &mut impl std::io::Write) {
        match self {
            Token::Keyword(s) => writeln!(out, "<keyword> {s} </keyword>").unwrap(),
            Token::Symbol(s) => match s {
                '<' => writeln!(out, "<symbol> &lt; </symbol>"),
                '>' => writeln!(out, "<symbol> &gt; </symbol>"),
                '&' => writeln!(out, "<symbol> &amp; </symbol>"),
                s => writeln!(out, "<symbol> {s} </symbol>"),
            }
            .unwrap(),
            Token::Identifier(s) => writeln!(out, "<identifier> {s} </identifier>").unwrap(),
            Token::IntegerConstant(s) => {
                writeln!(out, "<integerConstant> {s} </integerConstant>").unwrap()
            }
            Token::StringConstant(s) => {
                writeln!(out, "<stringConstant> {s} </stringConstant>").unwrap()
            }
        }
    }
}

impl<'a> Token<'a> {
    fn from(s: &'a str) -> Self {
        let (token, rest) = Self::parse_next(s).unwrap();
        assert!(rest.is_empty());
        token
    }

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
        let result = strip_prefix_and_rest(s, keyword);
        if result.is_some() {
            return result;
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

    pub fn peek(self: &mut Self) -> Option<&Token<'_>> {
        self.inner.peek()
    }
}

struct InnerTokenStream<'a> {
    rest: &'a str,
}

impl<'a> TokenStream<'a> {
    pub fn next_assert(self: &mut Self, s: &str) -> Token<'a> {
        let token = self.next().unwrap();
        assert_eq!(token, Token::from(s));
        token
    }
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
