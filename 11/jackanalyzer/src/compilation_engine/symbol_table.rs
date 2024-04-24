use std::collections::HashMap;

type Name = str;
type Index = usize;
type IdentType = str;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum IdentCat {
    Field,
    Static,
    Var,
    Arg,
}

#[derive(Debug)]
pub struct SymbolTable<'a> {
    inner: HashMap<(IdentCat, &'a Name), (&'a IdentType, Index)>,
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

    pub fn insert(&mut self, name: &'a str, cat: IdentCat, typ: &'a IdentType) {
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

    pub fn n_fields(&self) -> usize {
        self.n_fields
    }

    pub fn n_vars(&self) -> usize {
        self.n_vars
    }
}
