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
    _uid: usize,
}

impl<'a, Writer: Write> CompilationEngine<'a, Writer> {
    fn new(out: &'a mut Writer, tokens: TokenStream<'a>, class_name: &'a str) -> Self {
        let sym = SymbolTable::new();
        CompilationEngine {
            out,
            tokens,
            sym,
            class_name,
            _uid: 0,
        }
    }

    fn compile_class(&mut self) -> Res {
        assert_eq!(self.tokens.unwrap_keyword(), "class");
        assert_eq!(self.tokens.unwrap_identifier(), self.class_name);
        assert_eq!(self.tokens.unwrap_symbol(), '{');

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

        assert_eq!(self.tokens.unwrap_symbol(), '}');
        assert!(
            self.tokens.next().is_none(),
            "Should be consumed after class."
        );
        Ok(())
    }

    fn compile_class_variable_declaration(self: &mut Self) -> Res {
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
                    assert_eq!(self.tokens.unwrap_symbol(), ',');
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, cat, typ.clone());
                }
                Symbol(';') => break,
                _ => return Err("Unexpected token multi class variable declaration"),
            }
        }
        assert_eq!(self.tokens.unwrap_symbol(), ';');

        Ok(())
    }

    fn compile_subroutine(self: &mut Self) -> Res {
        let proc_cat = self.tokens.unwrap_keyword();
        assert!(matches!(proc_cat, "constructor" | "method" | "function"));
        let return_type = self.tokens.next().unwrap();
        let proc_name = self.tokens.unwrap_identifier();

        if proc_cat == "method" {
            self.sym.insert("this", IdentCat::Arg, return_type.clone())
        }

        assert_eq!(self.tokens.unwrap_symbol(), '(');
        loop {
            match self.tokens.peek().unwrap() {
                Keyword("int") | Keyword("char") | Identifier(_) => {
                    let typ = self.tokens.next().unwrap();
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, IdentCat::Arg, typ);
                }
                Symbol(',') => {
                    self.tokens.next().unwrap();
                }
                Symbol(')') => break,
                _ => return Err("Unexpected token in parameter list"),
            }
        }
        assert_eq!(self.tokens.unwrap_symbol(), ')');

        assert_eq!(self.tokens.unwrap_symbol(), '{');
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

        assert_eq!(self.tokens.unwrap_symbol(), '}');

        Ok(())
    }

    fn compile_statements(self: &mut Self) -> Res {
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
        Ok(())
    }

    fn compile_return(self: &mut Self) -> Res {
        assert_eq!(self.tokens.unwrap_keyword(), "return");
        if matches!(self.tokens.peek().unwrap(), Symbol(';')) {
            // Return dummy value in `void` case
            writeln! {self.out, "push constant 0"}.unwrap();
        } else {
            self.compile_expression()?;
        }
        assert_eq!(self.tokens.unwrap_symbol(), ';');
        writeln!(self.out, "return").unwrap();

        Ok(())
    }

    fn compile_do(self: &mut Self) -> Res {
        assert_eq!(self.tokens.unwrap_keyword(), "do");
        self.compile_term_inner()?;
        assert_eq!(self.tokens.unwrap_symbol(), ';');
        writeln!(self.out, "pop temp 0").unwrap(); // Yank computed value

        Ok(())
    }

    fn compile_while(self: &mut Self) -> Res {
        let label_start = self.create_label("WHILE_EXP");
        let label_end = self.create_label("WHILE_END");
        assert_eq!(self.tokens.unwrap_keyword(), "while");
        writeln!(self.out, "label {label_start}").unwrap();
        assert_eq!(self.tokens.unwrap_symbol(), '(');
        self.compile_expression()?;
        assert_eq!(self.tokens.unwrap_symbol(), ')');
        writeln!(self.out, "not").unwrap();
        writeln!(self.out, "if-goto {label_end}").unwrap();
        assert_eq!(self.tokens.unwrap_symbol(), '{');
        self.compile_statements()?;
        assert_eq!(self.tokens.unwrap_symbol(), '}');
        writeln!(self.out, "goto {label_start}").unwrap();
        writeln!(self.out, "label {label_end}").unwrap();

        Ok(())
    }

    fn compile_if(self: &mut Self) -> Res {
        let label_else = self.create_label("IF_FALSE");
        let label_end = self.create_label("IF_TRUE");

        assert_eq!(self.tokens.unwrap_keyword(), "if");
        assert_eq!(self.tokens.unwrap_symbol(), '(');
        self.compile_expression()?;
        assert_eq!(self.tokens.unwrap_symbol(), ')');
        writeln!(self.out, "not").unwrap();
        writeln!(self.out, "if-goto {label_else}").unwrap();

        assert_eq!(self.tokens.unwrap_symbol(), '{');
        self.compile_statements()?;
        assert_eq!(self.tokens.unwrap_symbol(), '}');
        writeln!(self.out, "goto {label_end}").unwrap();

        writeln!(self.out, "label {label_else}",).unwrap();
        if matches!(self.tokens.peek().unwrap(), Keyword("else")) {
            assert_eq!(self.tokens.unwrap_keyword(), "else");
            assert_eq!(self.tokens.unwrap_symbol(), '{');
            self.compile_statements()?;
            assert_eq!(self.tokens.unwrap_symbol(), '}');
        }
        writeln!(self.out, "label {label_end}").unwrap();

        Ok(())
    }

    fn compile_let(self: &mut Self) -> Res {
        assert_eq!(self.tokens.unwrap_keyword(), "let");
        let dest_ident = self.tokens.unwrap_identifier();
        if matches!(self.tokens.peek().unwrap(), Symbol('[')) {
            // TODO
            assert_eq!(self.tokens.unwrap_symbol(), '[');
            self.compile_expression()?;
            assert_eq!(self.tokens.unwrap_symbol(), ']');
        }
        assert_eq!(self.tokens.unwrap_symbol(), '=');
        self.compile_expression()?;
        assert_eq!(self.tokens.unwrap_symbol(), ';');
        self.pop(dest_ident);
        Ok(())
    }

    fn compile_term(self: &mut Self) -> Res {
        self.compile_term_inner()?;
        Ok(())
    }

    fn compile_term_inner(self: &mut Self) -> Res {
        let t1 = self.tokens.next().unwrap();
        Ok(match (&t1, self.tokens.peek().unwrap()) {
            (Keyword("true"), _) => writeln!(self.out, "push constant 1\nneg").unwrap(),
            (Keyword("false") | Keyword("null"), _) => {
                writeln!(self.out, "push constant 0").unwrap()
            }
            (Keyword("this"), _) => writeln!(self.out, "push pointer 0").unwrap(),
            (IntegerConstant(i), _) => writeln!(self.out, "push constant {i}").unwrap(),
            (StringConstant(_), _) => todo!(),
            (Identifier(_), Symbol('[')) => {
                assert_eq!(self.tokens.unwrap_symbol(), '[');
                self.compile_expression()?;
                assert_eq!(self.tokens.unwrap_symbol(), ']');
                todo!();
            }
            (Symbol('('), _) => {
                self.compile_expression()?;
                assert_eq!(self.tokens.unwrap_symbol(), ')');
            }
            (Symbol('-'), _) => {
                self.compile_term()?;
                writeln!(self.out, "neg").unwrap()
            }
            (Symbol('~'), _) => {
                self.compile_term()?;
                writeln!(self.out, "not").unwrap()
            }
            (Identifier(function_name), Symbol('(')) => {
                assert_eq!(self.tokens.unwrap_symbol(), '(');
                let n_args = self.compile_expression_list()?;
                assert_eq!(self.tokens.unwrap_symbol(), ')');
                let class_name = self.class_name;
                writeln!(self.out, "call {class_name}.{function_name} {n_args}").unwrap()
            }
            (Identifier(class_name), Symbol('.')) => {
                assert_eq!(self.tokens.unwrap_symbol(), '.');
                let method_name = self.tokens.unwrap_identifier();
                assert_eq!(self.tokens.unwrap_symbol(), '(');
                let n_args = self.compile_expression_list()?;
                assert_eq!(self.tokens.unwrap_symbol(), ')');
                writeln!(self.out, "call {class_name}.{method_name} {n_args}").unwrap();
            }
            (Identifier(ident_name), _) => {
                self.push(ident_name);
            }
            _ => return Err("Unexpected token in term"),
        })
    }

    fn compile_expression_list(self: &mut Self) -> Result<usize, &'static str> {
        let mut n = 0;
        loop {
            match self.tokens.peek().unwrap() {
                Symbol(',') => {
                    assert_eq!(self.tokens.unwrap_symbol(), ',');
                }
                Symbol(')') => break,
                _ => {
                    n += 1;
                    self.compile_expression()?
                }
            }
        }
        Ok(n)
    }

    fn compile_expression(self: &mut Self) -> Res {
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
        Ok(())
    }
    fn compile_variable_declaration(self: &mut Self) -> Res {
        assert!(matches!(self.tokens.next().unwrap(), Keyword("var")));
        let typ = self.tokens.next().unwrap();
        let name = self.tokens.unwrap_identifier();
        self.sym.insert(name, IdentCat::Var, typ.clone());
        loop {
            match self.tokens.peek().unwrap() {
                Symbol(',') => {
                    assert_eq!(self.tokens.unwrap_symbol(), ',');
                    let name = self.tokens.unwrap_identifier();
                    self.sym.insert(name, IdentCat::Var, typ.clone());
                }
                Symbol(';') => {
                    assert_eq!(self.tokens.unwrap_symbol(), ';');
                    break;
                }
                _ => return Err("Unexpected token in variable declaration"),
            }
        }
        Ok(())
    }

    pub fn push(&mut self, ident_name: &str) {
        let (cat, _typ, idx) = self.sym.retrieve(ident_name);
        writeln!(self.out, "push {cat} {idx}").unwrap();
    }

    pub fn pop(&mut self, ident_name: &str) {
        let (cat, _typ, idx) = self.sym.retrieve(ident_name);
        writeln!(self.out, "pop {cat} {idx}").unwrap();
    }

    pub fn create_label(&mut self, prefix: &str) -> String {
        let uid = self.uid();
        let label_name = format!("{prefix}{uid}");
        label_name
    }

    pub fn uid(&mut self) -> usize {
        let out = self._uid;
        self._uid += 1;
        return out;
    }
}
