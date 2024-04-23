use std::{
    fs::{self, File},
    io::BufWriter,
    iter::Peekable,
    path::Path,
};

use crate::token::{
    token_stream,
    Token::{self, *},
};
type Res = Result<(), &'static str>;

pub(crate) fn compile_path(path: &Path) -> std::io::Result<()> {
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

fn compile_file(jack_file: &Path, out: &mut impl std::io::Write) {
    // let module_id = jack_file
    //     .file_stem()
    //     .unwrap_or_else(|| panic!("Expected *.jack file, got `{}`", jack_file.display()))
    //     .to_str()
    //     .expect("Filename has to be unicode.");
    let s = fs::read_to_string(jack_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", jack_file.display()));

    let filtered = filter_comments(&s);

    let mut tokens = token_stream(&filtered).peekable();
    compile_class(out, &mut tokens).unwrap_or_else(|e| {
        out.flush().unwrap();
        panic!("Compilation failed: {e} ({})", jack_file.display());
    });
}

pub(crate) fn filter_comments(s: &str) -> String {
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

pub(crate) fn compile_class<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<class>").unwrap();
    tokens.next().unwrap().write_xml(out); // class
    tokens.next().unwrap().write_xml(out); // Identifier
    tokens.next().unwrap().write_xml(out); // {

    loop {
        match tokens.peek().unwrap() {
            Keyword("field") | Keyword("static") => {
                compile_class_variable_declaration(out, tokens)?
            }
            Keyword("constructor") | Keyword("method") | Keyword("function") => {
                break;
            }
            _ => return Err("Unexpected token in class variable declaration"),
        }
    }

    loop {
        match tokens.peek().unwrap() {
            Keyword("constructor") | Keyword("method") | Keyword("function") => {
                compile_subroutine(out, tokens)?
            }
            Symbol('}') => break,
            _ => return Err("Unexpected token in class subroutine declaration"),
        }
    }

    tokens.next().unwrap().write_xml(out); // }
    writeln!(out, "</class>").unwrap();
    assert!(tokens.next().is_none(), "Should be consumed after class.");
    Ok(())
}

fn compile_class_variable_declaration<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<classVarDec>").unwrap();
    tokens.next().unwrap().write_xml(out); // field | static
    tokens.next().unwrap().write_xml(out); // type
    tokens.next().unwrap().write_xml(out); // identifier
    loop {
        match tokens.peek().unwrap() {
            Symbol(',') => {
                tokens.next().unwrap().write_xml(out); // ,
                tokens.next().unwrap().write_xml(out); // identifier
            }
            Symbol(';') => break,
            _ => return Err("Unexpected token multi class variable declaration"),
        }
    }
    tokens.next().unwrap().write_xml(out); // ;

    writeln!(out, "</classVarDec>").unwrap();
    Ok(())
}
fn compile_subroutine<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<subroutineDec>").unwrap();
    tokens.next().unwrap().write_xml(out); // constructor | function | method
    tokens.next().unwrap().write_xml(out); // type
    tokens.next().unwrap().write_xml(out); // identifier
    tokens.next().unwrap().write_xml(out); // (
    writeln!(out, "<parameterList>").unwrap();
    loop {
        match tokens.peek().unwrap() {
            Keyword("int") | Keyword("char") | Identifier(_) => {
                tokens.next().unwrap().write_xml(out); // type
                tokens.next().unwrap().write_xml(out); // identifier
            }
            Symbol(',') => tokens.next().unwrap().write_xml(out),
            Symbol(')') => break,
            _ => return Err("Unexpected token in parameter list"),
        }
    }
    writeln!(out, "</parameterList>").unwrap();
    tokens.next().unwrap().write_xml(out); // )

    writeln!(out, "<subroutineBody>").unwrap();
    tokens.next().unwrap().write_xml(out); // {
    while matches!(tokens.peek().unwrap(), Keyword("var")) {
        compile_variable_declaration(out, tokens)?;
    }
    compile_statements(out, tokens)?;
    tokens.next().unwrap().write_xml(out); // }
    writeln!(out, "</subroutineBody>").unwrap();

    writeln!(out, "</subroutineDec>").unwrap();
    Ok(())
}

fn compile_statements<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<statements>").unwrap();
    loop {
        match tokens.peek().unwrap() {
            Keyword("let") => {
                writeln!(out, "<letStatement>").unwrap();
                tokens.next().unwrap().write_xml(out); // let
                tokens.next().unwrap().write_xml(out); // identifier
                if matches!(tokens.peek().unwrap(), Symbol('[')) {
                    tokens.next().unwrap().write_xml(out); // [
                    compile_expression(out, tokens)?;
                    tokens.next().unwrap().write_xml(out); // ]
                }
                tokens.next().unwrap().write_xml(out); // =
                compile_expression(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // ;
                writeln!(out, "</letStatement>").unwrap();
            }
            Keyword("if") => {
                writeln!(out, "<ifStatement>").unwrap();
                tokens.next().unwrap().write_xml(out); // if
                tokens.next().unwrap().write_xml(out); // (
                compile_expression(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // )
                tokens.next().unwrap().write_xml(out); // {
                compile_statements(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // }
                if matches!(tokens.peek().unwrap(), Keyword("else")) {
                    tokens.next().unwrap().write_xml(out); // else
                    tokens.next().unwrap().write_xml(out); // {
                    compile_statements(out, tokens)?;
                    tokens.next().unwrap().write_xml(out); // }
                }
                writeln!(out, "</ifStatement>").unwrap();
            }
            Keyword("while") => {
                writeln!(out, "<whileStatement>").unwrap();
                tokens.next().unwrap().write_xml(out); // while
                tokens.next().unwrap().write_xml(out); // (
                compile_expression(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // )
                tokens.next().unwrap().write_xml(out); // {
                compile_statements(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // }
                writeln!(out, "</whileStatement>").unwrap();
            }
            Keyword("do") => {
                writeln!(out, "<doStatement>").unwrap();
                tokens.next().unwrap().write_xml(out); // do
                compile_term_inner(out, tokens)?;
                tokens.next().unwrap().write_xml(out); // ;
                writeln!(out, "</doStatement>").unwrap();
            }
            Keyword("return") => {
                writeln!(out, "<returnStatement>").unwrap();
                tokens.next().unwrap().write_xml(out); // return
                if !matches!(tokens.peek().unwrap(), Symbol(';')) {
                    compile_expression(out, tokens)?;
                }
                tokens.next().unwrap().write_xml(out); // ;
                writeln!(out, "</returnStatement>").unwrap();
            }
            Symbol('}') => break,
            _ => return Err("Unexpected token in statements"),
        }
    }
    writeln!(out, "</statements>").unwrap();
    Ok(())
}

fn compile_term<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<term>").unwrap();
    compile_term_inner(out, tokens)?;
    writeln!(out, "</term>").unwrap();
    Ok(())
}

fn compile_term_inner<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    let t1 = tokens.next().unwrap();
    t1.write_xml(out);
    Ok(match (&t1, tokens.peek().unwrap()) {
        (
            IntegerConstant(_) | StringConstant(_) | Keyword("true") | Keyword("false")
            | Keyword("null") | Keyword("this"),
            _,
        ) => (),
        (Identifier(_), Symbol('[')) => {
            tokens.next().unwrap().write_xml(out); // [
            compile_expression(out, tokens)?;
            tokens.next().unwrap().write_xml(out); // ]
        }
        (Symbol('('), _) => {
            compile_expression(out, tokens)?;
            tokens.next().unwrap().write_xml(out); // )
        }
        (Symbol('-') | Symbol('~'), _) => {
            compile_term(out, tokens)?;
        }
        (Identifier(_), Symbol('(')) => {
            tokens.next().unwrap().write_xml(out); // (
            compile_expression_list(out, tokens)?;
            tokens.next().unwrap().write_xml(out); // )
        }
        (Identifier(_), Symbol('.')) => {
            tokens.next().unwrap().write_xml(out); // .
            tokens.next().unwrap().write_xml(out); // identifier
            tokens.next().unwrap().write_xml(out); // (
            compile_expression_list(out, tokens)?;
            tokens.next().unwrap().write_xml(out); // )
        }
        (Identifier(_), _) => (),
        _ => return Err("Unexpected token in term"),
    })
}

fn compile_expression_list<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<expressionList>").unwrap();
    loop {
        match tokens.peek().unwrap() {
            Symbol(',') => {
                tokens.next().unwrap().write_xml(out); // ,
            }
            Symbol(')') => break,
            _ => compile_expression(out, tokens)?,
        }
    }
    writeln!(out, "</expressionList>").unwrap();
    Ok(())
}

fn compile_expression<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<expression>").unwrap();
    compile_term(out, tokens)?;
    while matches!(
        tokens.peek().unwrap(),
        Symbol('+')
            | Symbol('-')
            | Symbol('*')
            | Symbol('/')
            | Symbol('&')
            | Symbol('|')
            | Symbol('<')
            | Symbol('>')
            | Symbol('=')
    ) {
        tokens.next().unwrap().write_xml(out);
        compile_term(out, tokens)?;
    }
    writeln!(out, "</expression>").unwrap();
    Ok(())
}
fn compile_variable_declaration<'a>(
    out: &mut impl std::io::Write,
    tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Res {
    writeln!(out, "<varDec>").unwrap();
    loop {
        match tokens.peek().unwrap() {
            Keyword("var") => {
                tokens.next().unwrap().write_xml(out); // var
                tokens.next().unwrap().write_xml(out); // type
                tokens.next().unwrap().write_xml(out); // identifier
            }
            Symbol(',') => {
                tokens.next().unwrap().write_xml(out); // ,
                tokens.next().unwrap().write_xml(out); // identifier
            }
            Symbol(';') => {
                tokens.next().unwrap().write_xml(out); // ;
                break;
            }
            _ => return Err("Unexpected token in variable declaration"),
        }
    }
    writeln!(out, "</varDec>").unwrap();
    Ok(())
}
