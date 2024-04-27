use std::collections::HashMap;

use crate::token::Token;

type Name = str;
type Index = usize;
type IdentType<'a> = Token<'a>; // type or ClassName
type Category = str;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum IdentCat {
    Field,
    Static,
    Var,
    Arg,
}

#[derive(Debug)]
pub struct SymbolTable<'a> {
    inner: HashMap<(IdentCat, &'a Name), (IdentType<'a>, Index)>,
    n_fields: usize,
    n_statics: usize,
    n_vars: usize,
    n_args: usize,
}

impl<'a> SymbolTable<'a> {
    pub fn new() -> Self {
        SymbolTable {
            inner: HashMap::new(),
            n_fields: 0,
            n_statics: 0,
            n_vars: 0,
            n_args: 0,
        }
    }

    pub fn insert(&mut self, name: &'a str, cat: IdentCat, typ: IdentType<'a>) {
        let idx = match cat {
            IdentCat::Field => {
                let tmp = self.n_fields;
                self.n_fields += 1;
                tmp
            }
            IdentCat::Static => {
                let tmp = self.n_statics;
                self.n_statics += 1;
                tmp
            }
            IdentCat::Var => {
                let tmp = self.n_vars;
                self.n_vars += 1;
                tmp
            }
            IdentCat::Arg => {
                let tmp = self.n_args;
                self.n_args += 1;
                tmp
            }
        };
        let old = self.inner.insert((cat, name), (typ, idx));
        assert!(old.is_none(), "Inserting {name} twice");
    }

    pub fn reset_vars_and_args(&mut self) {
        self.inner.retain(|(cat, _), _| match cat {
            IdentCat::Field | IdentCat::Static => true,
            IdentCat::Var | IdentCat::Arg => false,
        });
        self.n_args = 0;
        self.n_vars = 0;
    }

    pub fn retrieve(&self, ident_name: &str) -> Option<(&'static Category, IdentType<'a>, Index)> {
        self.inner
            .iter()
            .find(|((cat, name), _)| {
                *name == ident_name && matches!(cat, IdentCat::Arg | IdentCat::Var)
            })
            .or_else(|| {
                self.inner
                    .iter()
                    .find(|((_cat, name), _)| *name == ident_name)
            })
            .map(|((cat, _name), (typ, idx))| {
                (
                    match cat {
                        IdentCat::Field => "this",
                        IdentCat::Static => "static",
                        IdentCat::Var => "local",
                        IdentCat::Arg => "argument",
                    },
                    typ.clone(),
                    *idx,
                )
            })
    }

    pub fn n_fields(&self) -> usize {
        self.n_fields
    }

    pub fn n_vars(&self) -> usize {
        self.n_vars
    }
}
