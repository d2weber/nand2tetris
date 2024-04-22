use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

fn main() {
    let mut args = env::args();
    let path = if args.len() == 1 {
        // Default to current directory
        env::current_dir().unwrap()
    } else if args.len() == 2 {
        let filename = args.next_back().unwrap();
        PathBuf::from(&filename)
    } else {
        panic!("Zero parameters or one parameter expected.");
    };
    compile_path(path.as_path()).unwrap();
}

#[cfg(test)]
mod test {
    use std::process::{Command, Stdio};

    use super::*;

    // #[test]
    // fn square_main() {
    //     let path = Path::new("../Square/Main.jack");
    //     let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    //     let path = cargo_root.join(path);
    //     compile_path(&path).unwrap();
    //     check_file(&path.with_extension("xml"));
    // }

    #[test]
    fn test_all_tokens() {
        check_tokens("../ArrayTest/Main.jack");
        check_tokens("../ExpressionLessSquare/SquareGame.jack");
        check_tokens("../Square/Main.jack");
        check_tokens("../Square/Square.jack");
        check_tokens("../ExpressionLessSquare/Main.jack");
        check_tokens("../ExpressionLessSquare/Square.jack");
        check_tokens("../Square/SquareGame.jack");
    }

    /// Create xml output file and compare it to reference
    fn check_tokens(jack_file: &str) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let jack_file = cargo_root.join(jack_file);
        let mut out_filename = jack_file.file_stem().unwrap().to_os_string();
        out_filename.push("T.xml");
        let out_filename = jack_file.with_file_name(out_filename);
        {
            let mut out = BufWriter::new(fs::File::create(out_filename.clone()).unwrap());
            let filtered = filter_comments(&fs::read_to_string(&jack_file).unwrap());
            writeln!(out, "<tokens>").unwrap();
            token_stream(&filtered).for_each(|t| t.write_xml(&mut out));
            writeln!(out, "</tokens>").unwrap();
        }
        // Make shure out is written
        check_file(&out_filename);
    }

    /// Checks if file is equal to reference in subfolder `reference`
    fn check_file(path: &Path) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);
        let reference = path
            .parent()
            .unwrap()
            .join("reference")
            .join(path.file_name().unwrap());
        assert!(
            Command::new("bash")
                .arg("../../../tools/TextComparer.sh")
                .arg(path)
                .arg(reference)
                .current_dir(cargo_root)
                .stdout(Stdio::null())
                .status()
                .expect("Failed to run TextComparer")
                .success(),
            "Bad status when running TextComparer"
        );
    }
}

fn compile_path(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        let mut out = BufWriter::new(File::create(path.with_extension("xml"))?);
        compile_file(path, &mut out);
        Ok(())
    } else if path.is_dir() {
        let name = path
            .file_name()
            .expect("Already checked that it's a directory");
        let out_file = path.join(name).with_extension("xml");
        let mut out = BufWriter::new(File::create(out_file)?);
        // TODO: Error when no jack file is found
        for dir_entry in fs::read_dir(path)? {
            let file = dir_entry?.path();
            if file.extension().is_some_and(|e| e == "jack") {
                compile_file(&file, &mut out)
            }
        }
        Ok(())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

fn compile_file(jack_file: &Path, out: &mut impl Write) {
    // let module_id = jack_file
    //     .file_stem()
    //     .unwrap_or_else(|| panic!("Expected *.jack file, got `{}`", jack_file.display()))
    //     .to_str()
    //     .expect("Filename has to be unicode.");
    let jack_file = fs::read_to_string(jack_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", jack_file.display()));

    let filtered = filter_comments(&jack_file);

    let tokens = token_stream(&filtered);
    // compileClass(out, tokens);
    // out.write_all(filtered.as_bytes())
    //     .expect("Failed to write output file");
}

// fn compileClass(out: impl Write, tokens: impl Iterator<Item = Token>) {
//     write!("<")
// }

#[derive(Debug)]
enum Token {
    Keyword(String),
    Symbol(char),
    Identifier(String),
    IntegerConstant(i32),
    StringConstant(String),
}
impl Token {
    #[cfg(test)]
    fn write_xml(&self, out: &mut impl Write) {
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

fn token_stream(s: &str) -> impl Iterator<Item = Token> + '_ {
    let mut rest = s;
    std::iter::from_fn(move || {
        rest = rest.trim();
        let first_char = rest.chars().next()?;
        Some(if let Some((kw, new_rest)) = keyword_token(rest) {
            rest = new_rest;
            Token::Keyword(kw.to_owned())
        } else {
            match first_char {
                '{' | '}' | '(' | ')' | '[' | ']' | '.' | ',' | ';' | '+' | '-' | '*' | '/'
                | '&' | '|' | '<' | '>' | '=' | '~' => {
                    rest = &rest[1..];
                    Token::Symbol(first_char)
                }
                '"' => {
                    let (string_val, new_rest) = rest
                        .strip_prefix('"')
                        .and_then(|s| s.split_once('"'))
                        .expect("String value not misses trailing `\"`");
                    rest = new_rest;
                    Token::StringConstant(string_val.to_owned())
                }
                _ if first_char.is_ascii_digit() => {
                    let idx = rest
                        .find(|c: char| !c.is_ascii_digit())
                        .expect("Trailing digit");
                    let v = &rest[..idx];
                    rest = &rest[idx..];
                    Token::IntegerConstant(v.parse().unwrap())
                }
                _ => {
                    let idx = rest
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .expect("Unterminated identifier");
                    let v = &rest[..idx];
                    rest = &rest[idx..];
                    Token::Identifier(v.to_owned())
                }
            }
        })
    })
}

fn filter_comments(s: &str) -> String {
    // Remove block comments
    let mut rest = s;
    let mut block_filtered = String::new();
    while let Some((first, comment_and_rest)) = rest.split_once("/*") {
        block_filtered += first;
        let (_comment, new_rest) = comment_and_rest
            .split_once("*/")
            .expect("Missing closing block comment");
        rest = new_rest;
    }
    block_filtered += rest;

    // Remove line comments
    let mut rest = block_filtered.as_str();
    let mut filtered = String::new();
    while let Some((first, comment_and_rest)) = rest.split_once("//") {
        filtered += first;
        filtered.push('\n'); // reinsert newline
        rest = if let Some((_comment, new_rest)) = comment_and_rest.split_once('\n') {
            new_rest
        } else {
            "" // Last line in string
        }
    }
    filtered + rest
}
