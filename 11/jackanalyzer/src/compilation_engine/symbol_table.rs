use std::collections::HashMap;

type Name = str;
type Index = usize;

pub enum IdentType<'a> {
    Int,
    Boolean,
    Char,
    Class(&'a str),
}

pub enum IdentCat {
    Field,
    Static,
    Var,
    Arg,
}

pub struct SymbolTable<'a> {
    inner: HashMap<&'a Name, (IdentType<'a>, IdentCat, Index)>,
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

    fn insert(self: &mut Self, name: &'a str, typ: IdentType<'a>, cat: IdentCat) {
        let idx: &mut Index = &mut match cat {
            IdentCat::Field => self.n_fields,
            IdentCat::Static => self.n_statics,
            IdentCat::Var => self.n_vars,
            IdentCat::Arg => self.n_args,
        };
        let old = self.inner.insert(name, (typ, cat, *idx));
        *idx += 1;
        assert!(old.is_none(), "Inserting {name} twice");
    }
}
