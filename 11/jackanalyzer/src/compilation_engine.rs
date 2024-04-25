mod symbol_table;

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use crate::{
    compilation_engine::symbol_table::IdentCat,
    token::{Token::*, TokenStream},
};

use self::symbol_table::SymbolTable;
type Res = Result<(), &'static str>;

pub(crate) fn compile_path(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        let mut out = BufWriter::new(File::create(path.with_extension("vm"))?);
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
                let out_file = path.join(name).with_extension("vm");
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
    let s = fs::read_to_string(jack_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", jack_file.display()));

    let filtered = filter_comments(&s);

    let tokens = TokenStream::new(&filtered);
    let class_name = jack_file.file_stem().unwrap().to_str().unwrap();
    CompilationEngine::new(out, tokens, class_name)
        .compile_class()
        .unwrap_or_else(|e| {
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

struct CompilationEngine<'a, Writer> {
    out: &'a mut Writer,
    tokens: TokenStream<'a>,
    sym: SymbolTable<'a>,
    class_name: &'a str,
}

impl<'a, Writer: Write> CompilationEngine<'a, Writer> {
    fn new(out: &'a mut Writer, tokens: TokenStream<'a>, class_name: &'a str) -> Self {
        let sym = SymbolTable::new();
        CompilationEngine {
            out,
            tokens,
            sym,
            class_name,
        }
    }

    fn compile_class(&mut self) -> Res {
        writeln!(self.out, "// <class>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out); // class
        self.tokens.next().unwrap().write_xml(self.out); // Identifier
        self.tokens.next().unwrap().write_xml(self.out); // {

        loop {
            match self.tokens.peek().unwrap() {
                Keyword("field") | Keyword("static") => {
                    self.compile_class_variable_declaration()?
                }
                Keyword("constructor") | Keyword("method") | Keyword("function") => {
                    break;
                }
                _ => return Err("Unexpected token in class variable declaration"),
            }
        }

        loop {
            match self.tokens.peek().unwrap() {
                Keyword("constructor") | Keyword("method") | Keyword("function") => {
                    self.compile_subroutine()?
                }
                Symbol('}') => break,
                _ => return Err("Unexpected token in class subroutine declaration"),
            }
            self.sym.reset_vars_and_args();
        }

        self.tokens.next().unwrap().write_xml(self.out); // }
        writeln!(self.out, "// </class>").unwrap();
        assert!(
            self.tokens.next().is_none(),
            "Should be consumed after class."
        );
        Ok(())
    }

    fn compile_class_variable_declaration(self: &mut Self) -> Res {
        writeln!(self.out, "// <classVarDec>").unwrap();
        let cat = match self.tokens.unwrap_keyword() {
            "field" => IdentCat::Field,
            "static" => IdentCat::Static,
            _ => return Err("Expected field or static"),
        };

        let typ = self.tokens.next().unwrap();
        let name = self.tokens.unwrap_identifier();
        self.sym.insert(name, cat, typ.clone());
        loop {
            match self.tokens.peek().unwrap() {
                Symbol(',') => {
                    self.tokens.next().unwrap().write_xml(self.out); // ,
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, cat, typ.clone());
                }
                Symbol(';') => break,
                _ => return Err("Unexpected token multi class variable declaration"),
            }
        }
        self.tokens.next().unwrap().write_xml(self.out); // ;

        writeln!(self.out, "// </classVarDec>").unwrap();
        Ok(())
    }

    fn compile_subroutine(self: &mut Self) -> Res {
        writeln!(self.out, "// <subroutineDec>").unwrap();
        let proc_cat = self.tokens.unwrap_keyword();
        assert!(matches!(proc_cat, "constructor" | "method" | "function"));
        let is_void = self.tokens.unwrap_keyword_or_identifier() == "void";
        let proc_name = self.tokens.unwrap_identifier();
        self.tokens.next().unwrap().write_xml(self.out); // (
        writeln!(self.out, "// <parameterList>").unwrap();
        loop {
            match self.tokens.peek().unwrap() {
                Keyword("int") | Keyword("char") | Identifier(_) => {
                    let typ = self.tokens.next().unwrap();
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, IdentCat::Arg, typ);
                }
                Symbol(',') => self.tokens.next().unwrap().write_xml(self.out),
                Symbol(')') => break,
                _ => return Err("Unexpected token in parameter list"),
            }
        }
        writeln!(self.out, "// </parameterList>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out); // )

        writeln!(self.out, "// <subroutineBody>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out); // {
        while matches!(self.tokens.peek().unwrap(), Keyword("var")) {
            self.compile_variable_declaration()?;
        }
        writeln!(
            self.out,
            "function {class_name}.{proc_name} {n_vars}",
            class_name = self.class_name,
            n_vars = self.sym.n_vars()
        )
        .unwrap();

        match proc_cat {
            "method" => {
                writeln!(self.out, "push argument 0").unwrap();
                writeln!(self.out, "pop pointer 0").unwrap();
            }
            "constructor" => {
                let n_fields = self.sym.n_fields();
                writeln!(self.out, "push constant {n_fields}",).unwrap();
                writeln!(self.out, "call Memory.alloc 1").unwrap();
                writeln!(self.out, "pop pointer 0").unwrap();
            }
            _ => (),
        }

        self.compile_statements()?;

        if is_void {
            writeln!(self.out, "push constant 0").unwrap();
        } else if proc_cat == "constructor" {
            writeln!(self.out, "push pointer 0").unwrap();
        }
        writeln!(self.out, "return").unwrap();

        self.tokens.next().unwrap().write_xml(self.out); // }
        writeln!(self.out, "// </subroutineBody>").unwrap();

        writeln!(self.out, "// </subroutineDec>").unwrap();
        Ok(())
    }

    fn compile_statements(self: &mut Self) -> Res {
        writeln!(self.out, "// <statements>").unwrap();
        loop {
            match self.tokens.peek().unwrap() {
                Keyword("let") => self.compile_let()?,
                Keyword("if") => self.compile_if()?,
                Keyword("while") => self.compile_while()?,
                Keyword("do") => self.compile_do()?,
                Keyword("return") => self.compile_return()?,
                Symbol('}') => break,
                _ => return Err("Unexpected token in statements"),
            }
        }
        writeln!(self.out, "// </statements>").unwrap();
        Ok(())
    }

    fn compile_return(self: &mut Self) -> Res {
        writeln!(self.out, "// <returnStatement>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out);
        if !matches!(self.tokens.peek().unwrap(), Symbol(';')) {
            self.compile_expression()?;
        }
        self.tokens.next().unwrap().write_xml(self.out);
        writeln!(self.out, "// </returnStatement>").unwrap();
        Ok(())
    }

    fn compile_do(self: &mut Self) -> Res {
        writeln!(self.out, "// <doStatement>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out); // do
        self.compile_term_inner()?;
        self.tokens.next().unwrap().write_xml(self.out); // ;
        writeln!(self.out, "pop temp 0").unwrap(); // Yank computed value

        writeln!(self.out, "// </doStatement>").unwrap();
        Ok(())
    }

    fn compile_while(self: &mut Self) -> Res {
        writeln!(self.out, "// <whileStatement>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out);
        self.tokens.next().unwrap().write_xml(self.out);
        self.compile_expression()?;
        self.tokens.next().unwrap().write_xml(self.out);
        self.tokens.next().unwrap().write_xml(self.out);
        self.compile_statements()?;
        self.tokens.next().unwrap().write_xml(self.out);
        writeln!(self.out, "// </whileStatement>").unwrap();
        Ok(())
    }

    fn compile_if(self: &mut Self) -> Res {
        writeln!(self.out, "// <ifStatement>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out);
        self.tokens.next().unwrap().write_xml(self.out);
        self.compile_expression()?;
        self.tokens.next().unwrap().write_xml(self.out);
        self.tokens.next().unwrap().write_xml(self.out);
        self.compile_statements()?;
        self.tokens.next().unwrap().write_xml(self.out);
        if matches!(self.tokens.peek().unwrap(), Keyword("else")) {
            self.tokens.next().unwrap().write_xml(self.out); // else
            self.tokens.next().unwrap().write_xml(self.out); // {
            self.compile_statements()?;
            self.tokens.next().unwrap().write_xml(self.out); // }
        }
        writeln!(self.out, "// </ifStatement>").unwrap();
        Ok(())
    }

    fn compile_let(self: &mut Self) -> Res {
        writeln!(self.out, "// <letStatement>").unwrap();
        self.tokens.next().unwrap().write_xml(self.out);
        self.tokens.next().unwrap().write_xml(self.out);
        if matches!(self.tokens.peek().unwrap(), Symbol('[')) {
            self.tokens.next().unwrap().write_xml(self.out); // [
            self.compile_expression()?;
            self.tokens.next().unwrap().write_xml(self.out); // ]
        }
        self.tokens.next().unwrap().write_xml(self.out);
        self.compile_expression()?;
        self.tokens.next().unwrap().write_xml(self.out);
        writeln!(self.out, "// </letStatement>").unwrap();
        Ok(())
    }

    fn compile_term(self: &mut Self) -> Res {
        writeln!(self.out, "// <term>").unwrap();
        self.compile_term_inner()?;
        writeln!(self.out, "// </term>").unwrap();
        Ok(())
    }

    fn compile_term_inner(self: &mut Self) -> Res {
        let t1 = self.tokens.next().unwrap();
        t1.write_xml(self.out);
        Ok(match (&t1, self.tokens.peek().unwrap()) {
            (Keyword("true"), _) => writeln!(self.out, "push constant 1\nneg").unwrap(),
            (Keyword("false") | Keyword("null"), _) => {
                writeln!(self.out, "push constant 0").unwrap()
            }
            (Keyword("this"), _) => writeln!(self.out, "push pointer 0").unwrap(),
            (IntegerConstant(i), _) => writeln!(self.out, "push constant {i}").unwrap(),
            (StringConstant(_), _) => todo!(),
            (Identifier(_), Symbol('[')) => {
                self.tokens.next().unwrap().write_xml(self.out); // [
                self.compile_expression()?;
                self.tokens.next().unwrap().write_xml(self.out); // ]
            }
            (Symbol('('), _) => {
                self.compile_expression()?;
                self.tokens.next().unwrap().write_xml(self.out); // )
            }
            (Symbol('-') | Symbol('~'), _) => {
                self.compile_term()?;
            }
            (Identifier(_), Symbol('(')) => {
                self.tokens.next().unwrap().write_xml(self.out); // (
                self.compile_expression_list()?;
                self.tokens.next().unwrap().write_xml(self.out); // )
            }
            (Identifier(_), Symbol('.')) => {
                self.tokens.next().unwrap().write_xml(self.out); // .
                let class_name = t1.unwrap_identifier();
                let method_name = self.tokens.unwrap_identifier();
                self.tokens.next().unwrap().write_xml(self.out); // (
                let n_args = self.compile_expression_list()?;
                self.tokens.next().unwrap().write_xml(self.out); // )
                writeln!(self.out, "call {class_name}.{method_name} {n_args}").unwrap();
            }
            (Identifier(ident_name), _) => {
                let (_cat, _typ, _idx) = self.sym.retrieve(ident_name);
                // match typ {
                //     Keyword(_) => (),
                //     Identifier(class_name) => {
                //         writeln!(self.out, "")

                //     }
                // }
            }
            _ => return Err("Unexpected token in term"),
        })
    }

    fn compile_expression_list(self: &mut Self) -> Result<usize, &'static str> {
        writeln!(self.out, "// <expressionList>").unwrap();
        let mut n = 0;
        loop {
            match self.tokens.peek().unwrap() {
                Symbol(',') => {
                    self.tokens.next().unwrap().write_xml(self.out); // ,
                }
                Symbol(')') => break,
                _ => {
                    n += 1;
                    self.compile_expression()?
                }
            }
        }
        writeln!(self.out, "// </expressionList>").unwrap();
        Ok(n)
    }

    fn compile_expression(self: &mut Self) -> Res {
        writeln!(self.out, "// <expression>").unwrap();
        self.compile_term()?;
        loop {
            let op = match self.tokens.peek().unwrap() {
                Symbol('+') => {
                    self.tokens.next().unwrap();
                    "add"
                }
                Symbol('-') => {
                    self.tokens.next().unwrap();
                    "sub"
                }
                Symbol('*') => {
                    self.tokens.next().unwrap();
                    "call Math.multiply 2"
                }
                Symbol('/') => {
                    self.tokens.next().unwrap();
                    "call Math.divide 2"
                }
                Symbol('&') => {
                    self.tokens.next().unwrap();
                    "and"
                }
                Symbol('|') => {
                    self.tokens.next().unwrap();
                    "or"
                }
                Symbol('<') => {
                    self.tokens.next().unwrap();
                    "lt"
                }
                Symbol('>') => {
                    self.tokens.next().unwrap();
                    "gt"
                }
                Symbol('=') => {
                    self.tokens.next().unwrap();
                    "eq"
                }
                _ => break,
            };
            self.compile_term()?;
            writeln!(self.out, "{op}").unwrap();
        }
        writeln!(self.out, "// </expression>").unwrap();
        Ok(())
    }
    fn compile_variable_declaration(self: &mut Self) -> Res {
        writeln!(self.out, "// <varDec>").unwrap();
        loop {
            match self.tokens.peek().unwrap() {
                Keyword("var") => {
                    assert!(matches!(self.tokens.next().unwrap(), Keyword("var")));
                    let typ = self.tokens.next().unwrap();
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, IdentCat::Var, typ);
                }
                Symbol(',') => {
                    self.tokens.next().unwrap().write_xml(self.out); // ,
                    self.tokens.next().unwrap().write_xml(self.out); // identifier
                }
                Symbol(';') => {
                    self.tokens.next().unwrap().write_xml(self.out); // ;
                    break;
                }
                _ => return Err("Unexpected token in variable declaration"),
            }
        }
        writeln!(self.out, "// </varDec>").unwrap();
        Ok(())
    }
}
