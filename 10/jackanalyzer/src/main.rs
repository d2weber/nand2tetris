use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    iter::Peekable,
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
    use std::{process::Command, str::from_utf8};

    use super::*;

    // #[test]
    // fn array_test() {
    //     check_folder("../ArrayTest");
    // }

    #[test]
    fn expressionless_square() {
        check_folder("../ExpressionLessSquare");
    }

    // #[test]
    // fn square() {
    //     check_folder("../Square");
    // }

    fn check_folder(s: &str) {
        let path = Path::new(s);
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);
        for file in files_with_extension(&path, "jack") {
            write_token_xml(&file);
        }
        compile_path(&path).unwrap();
        for file in files_with_extension(&path.join("reference"), "xml") {
            check_file(&file)
        }
    }

    fn files_with_extension<'a>(
        path: &'a Path,
        extension: &'a str,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        fs::read_dir(&path).unwrap().filter_map(move |dir_entry| {
            let file = dir_entry.unwrap().path();
            file.extension()
                .is_some_and(|e| e == extension)
                .then_some(file)
        })
    }

    fn write_token_xml(jack_file: &Path) -> PathBuf {
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
        out_filename
    }

    /// Checks that reference is equal to file in parent folder
    fn check_file(reference: &Path) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let reference = cargo_root.join(reference);
        let path = reference
            .ancestors()
            .nth(2)
            .unwrap()
            .join(reference.file_name().unwrap());
        let output = Command::new("bash")
            .arg("../../../tools/TextComparer.sh")
            .arg(path)
            .arg(&reference)
            .current_dir(cargo_root)
            .output()
            .expect("Failed to run TextComparer");
        assert!(
            output.status.success(),
            "TextComparer failed for reference {}: {}{}",
            reference.display(),
            from_utf8(&output.stderr).unwrap(),
            from_utf8(&output.stdout).unwrap()
        );
    }
}

fn compile_path(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        let mut out = BufWriter::new(File::create(path.with_extension("xml"))?);
        compile_file(path, &mut out);
        Ok(())
    } else if path.is_dir() {
        // TODO: Error when no jack file is found
        for dir_entry in fs::read_dir(path)? {
            let jack_file = dir_entry?.path();
            if jack_file.extension().is_some_and(|e| e == "jack") {
                let name = jack_file
                    .file_name()
                    .expect("Already checked that it's a file");
                let out_file = path.join(name).with_extension("xml");
                let mut out = BufWriter::new(File::create(out_file)?);
                compile_file(&jack_file, &mut out)
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

    let mut tokens = token_stream(&filtered).peekable();
    compile_class(out, &mut tokens).unwrap();
    // out.write_all(filtered.as_bytes())
    //     .expect("Failed to write output file");
}

type Res = Result<(), &'static str>;

fn compile_class(out: &mut impl Write, tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Res {
    writeln!(out, "<class>").unwrap();
    tokens.next().unwrap().write_xml(out); // class
    tokens.next().unwrap().write_xml(out); // Identifier
    tokens.next().unwrap().write_xml(out); // {

    // match tokens.peek().unwrap() {
    //     Token::Keyword(_) => todo!(),
    //     Token::Symbol(_) => todo!(),
    //     Token::Identifier(_) => todo!(),
    //     Token::IntegerConstant(_) => todo!(),
    //     Token::StringConstant(_) => todo!(),
    // }
    // compile_class_variable_declaration(out, tokens)?;

    writeln!(out, "</class>").unwrap();
    Ok(())
}

fn compile_class_variable_declaration(
    out: &mut impl Write,
    tokens: &mut impl Iterator<Item = Token>,
) -> Res {
    writeln!(out, "<classVarDec>").unwrap();

    writeln!(out, "</classVarDec>").unwrap();
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Token {
    Keyword(String),
    Symbol(char),
    Identifier(String),
    IntegerConstant(i32),
    StringConstant(String),
}
impl Token {
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
            Token::StringConstant(string_const.to_owned())
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
            Token::Identifier(v.to_owned())
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
