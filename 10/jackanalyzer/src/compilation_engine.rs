use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use crate::token::{Token::*, TokenStream};
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

fn compile_file(jack_file: &Path, out: &mut impl Write) {
    // let module_id = jack_file
    //     .file_stem()
    //     .unwrap_or_else(|| panic!("Expected *.jack file, got `{}`", jack_file.display()))
    //     .to_str()
    //     .expect("Filename has to be unicode.");
    let s = fs::read_to_string(jack_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", jack_file.display()));

    let filtered = filter_comments(&s);

    let mut tokens = TokenStream::new(&filtered);
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

pub(crate) fn compile_class<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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

fn compile_class_variable_declaration<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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
fn compile_subroutine<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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

fn compile_statements(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
    writeln!(out, "<statements>").unwrap();
    loop {
        match tokens.peek().unwrap() {
            Keyword("let") => compile_let(out, tokens)?,
            Keyword("if") => compile_if(out, tokens)?,
            Keyword("while") => compile_while(out, tokens)?,
            Keyword("do") => compile_do(out, tokens)?,
            Keyword("return") => compile_return(out, tokens)?,
            Symbol('}') => break,
            _ => return Err("Unexpected token in statements"),
        }
    }
    writeln!(out, "</statements>").unwrap();
    Ok(())
}

fn compile_return(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
    writeln!(out, "<returnStatement>").unwrap();
    tokens.next().unwrap().write_xml(out);
    if !matches!(tokens.peek().unwrap(), Symbol(';')) {
        compile_expression(out, tokens)?;
    }
    tokens.next().unwrap().write_xml(out);
    writeln!(out, "</returnStatement>").unwrap();
    Ok(())
}

fn compile_do(out: &mut impl Write, tokens: &mut TokenStream<'_>) -> Res {
    writeln!(out, "<doStatement>").unwrap();
    tokens.next().unwrap().write_xml(out);
    compile_term_inner(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    writeln!(out, "</doStatement>").unwrap();
    Ok(())
}

fn compile_while(out: &mut impl Write, tokens: &mut TokenStream<'_>) -> Res {
    writeln!(out, "<whileStatement>").unwrap();
    tokens.next().unwrap().write_xml(out);
    tokens.next().unwrap().write_xml(out);
    compile_expression(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    tokens.next().unwrap().write_xml(out);
    compile_statements(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    writeln!(out, "</whileStatement>").unwrap();
    Ok(())
}

fn compile_if(out: &mut impl Write, tokens: &mut TokenStream<'_>) -> Res {
    writeln!(out, "<ifStatement>").unwrap();
    tokens.next().unwrap().write_xml(out);
    tokens.next().unwrap().write_xml(out);
    compile_expression(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    tokens.next().unwrap().write_xml(out);
    compile_statements(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    if matches!(tokens.peek().unwrap(), Keyword("else")) {
        tokens.next().unwrap().write_xml(out); // else
        tokens.next().unwrap().write_xml(out); // {
        compile_statements(out, tokens)?;
        tokens.next().unwrap().write_xml(out); // }
    }
    writeln!(out, "</ifStatement>").unwrap();
    Ok(())
}

fn compile_let(out: &mut impl Write, tokens: &mut TokenStream<'_>) -> Res {
    writeln!(out, "<letStatement>").unwrap();
    tokens.next().unwrap().write_xml(out);
    tokens.next().unwrap().write_xml(out);
    if matches!(tokens.peek().unwrap(), Symbol('[')) {
        tokens.next().unwrap().write_xml(out); // [
        compile_expression(out, tokens)?;
        tokens.next().unwrap().write_xml(out); // ]
    }
    tokens.next().unwrap().write_xml(out);
    compile_expression(out, tokens)?;
    tokens.next().unwrap().write_xml(out);
    writeln!(out, "</letStatement>").unwrap();
    Ok(())
}

fn compile_term<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
    writeln!(out, "<term>").unwrap();
    compile_term_inner(out, tokens)?;
    writeln!(out, "</term>").unwrap();
    Ok(())
}

fn compile_term_inner<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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
    out: &mut impl Write,
    tokens: &mut TokenStream,
) -> Result<usize, &'static str> {
    writeln!(out, "<expressionList>").unwrap();
    let mut n = 0;
    loop {
        match tokens.peek().unwrap() {
            Symbol(',') => {
                tokens.next().unwrap().write_xml(out); // ,
            }
            Symbol(')') => break,
            _ => {
                n += 1;
                compile_expression(out, tokens)?
            }
        }
    }
    writeln!(out, "</expressionList>").unwrap();
    Ok(n)
}

fn compile_expression<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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
fn compile_variable_declaration<'a>(out: &mut impl Write, tokens: &mut TokenStream) -> Res {
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
