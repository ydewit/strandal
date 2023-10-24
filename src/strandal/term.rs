use std::fmt::Display;

use crate::strandal::store::Store;

use super::{
    cell::{Cell, CellPtr},
    equation::VarPort,
    var::{Var, VarPtr},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TermPtr {
    CellPtr(CellPtr),
    VarPtr(VarPtr),
}

#[derive(Debug)]
pub enum Term {
    Var(Var),
    Cell(Cell),
}

impl From<VarPtr> for TermPtr {
    fn from(value: VarPtr) -> Self {
        TermPtr::VarPtr(value)
    }
}

impl From<VarPort> for TermPtr {
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

pub struct TermPtrDisplay<'a> {
    pub(crate) term_ptr: &'a TermPtr,
    pub(crate) store: &'a Store,
}
impl<'a> Display for TermPtrDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.term_ptr {
            TermPtr::CellPtr(cell_ptr) => {
                write!(f, "{}", self.store.display_cell(cell_ptr),)
            }
            TermPtr::VarPtr(var_ptr) => write!(f, "{}", var_ptr),
        }
    }
}
