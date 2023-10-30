use std::fmt::Display;

use crate::strandal::store::Store;

use super::{
    cell::{Cell, CellRef},
    equation::VarPort,
    var::{Var, VarRef},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TermRef {
    CellRef(CellRef),
    VarRef(VarRef),
}

#[derive(Debug)]
pub enum Term {
    Var(Var),
    Cell(Cell),
}

impl From<VarRef> for TermRef {
    fn from(value: VarRef) -> Self {
        TermRef::VarRef(value)
    }
}

impl From<VarPort> for TermRef {
    fn from(value: VarPort) -> Self {
        value.ptr.into()
    }
}

impl From<Cell> for Term {
    fn from(cell: Cell) -> Self {
        Term::Cell(cell)
    }
}

impl From<Var> for Term {
    fn from(var: Var) -> Self {
        Term::Var(var)
    }
}

pub struct TermRefDisplay<'a> {
    pub(crate) term_ref: &'a TermRef,
    pub(crate) store: &'a Store,
}
impl<'a> Display for TermRefDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.term_ref {
            TermRef::CellRef(cell_ref) => {
                write!(f, "{}", self.store.display_cell(cell_ref),)
            }
            TermRef::VarRef(var_ref) => write!(f, "{}", var_ref),
        }
    }
}
