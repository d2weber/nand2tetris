#[derive(Debug, PartialEq)]
pub(crate) enum Token<'a> {
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

pub(crate) fn token_stream(s: &str) -> impl Iterator<Item = Token> + '_ {
    let mut rest = s;
    std::iter::from_fn(move || {
        rest = rest.trim();
        let first_char = rest.chars().next()?;
        Some(if let Some((kw, new_rest)) = keyword_token(rest) {
            rest = new_rest;
            Token::Keyword(kw)
        } else if let Some(new_rest) = rest.strip_prefix([
            '{', '}', '(', ')', '[', ']', '.', ',', ';', '+', '-', '*', '/', '&', '|', '<', '>',
            '=', '~',
        ]) {
            rest = new_rest;
            Token::Symbol(first_char)
        } else if let Some(new_rest) = rest.strip_prefix('"') {
            let string_const;
            (string_const, rest) = new_rest
                .split_once('"')
                .expect("String value not misses trailing `\"`");
            Token::StringConstant(string_const)
        } else if first_char.is_ascii_digit() {
            let idx = rest
                .find(|c: char| !c.is_ascii_digit())
                .expect("Trailing digit");
            let v = &rest[..idx];
            rest = &rest[idx..];
            Token::IntegerConstant(v.parse().unwrap())
        } else {
            let idx = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .expect("Unterminated identifier");
            let v = &rest[..idx];
            rest = &rest[idx..];
            Token::Identifier(v)
        })
    })
}
