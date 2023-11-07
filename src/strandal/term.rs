use super::{
    store::Ptr,
    var::{Var, VarUse},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermPtr {
    Era,
    Ptr(Ptr),
}
unsafe impl Send for TermPtr {}
unsafe impl Sync for TermPtr {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Dup(Option<(TermPtr, TermPtr)>, Option<Ptr>),
    App(Option<(TermPtr, TermPtr)>),
    Lam(Option<(TermPtr, TermPtr)>),
}

unsafe impl Send for Cell {}
unsafe impl Sync for Cell {}

#[derive(Debug, PartialEq, Eq)]
pub enum Term {
    Var(Var),
    Cell(Cell),
}
impl<'a> TryFrom<&'a Term> for &'a Cell {
    type Error = &'a Term;

    fn try_from(value: &'a Term) -> Result<Self, Self::Error> {
        match value {
            Term::Cell(cell) => Ok(cell),
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a Term> for &'a Var {
    type Error = &'a Term;

    fn try_from(value: &'a Term) -> Result<Self, Self::Error> {
        match value {
            Term::Var(var) => Ok(var),
            _ => Err(value),
        }
    }
}

impl From<VarUse> for TermPtr {
    fn from(value: VarUse) -> Self {
        TermPtr::Ptr(value.ptr())
    }
}
