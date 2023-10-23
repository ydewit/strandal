use std::fmt::Display;

use crate::icomb::store::{Ptr, Store};

use super::term::{Term, TermPtr};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellPtr {
    Era,
    Ref(Ptr<CellPtr>),
}
impl CellPtr {
    #[inline]
    pub fn get_ptr(&self) -> Option<&Ptr<CellPtr>> {
        match self {
            CellPtr::Era => None,
            CellPtr::Ref(ptr) => Some(ptr),
        }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Ctr(TermPtr, TermPtr),
    Dup(TermPtr, TermPtr),
}

impl Cell {
    pub fn is_ctr(&self) -> bool {
        match self {
            Cell::Ctr(_, _) => true,
            _ => false,
        }
    }

    pub fn is_dup(&self) -> bool {
        match self {
            Cell::Dup(_, _) => true,
            _ => false,
        }
    }

    pub fn ports(&self) -> (&TermPtr, &TermPtr) {
        match self {
            Cell::Ctr(port_0, port_1) => (port_0, port_1),
            Cell::Dup(port_0, port_1) => (port_0, port_1),
        }
    }
}


impl TryFrom<u32> for CellPtr {
    type Error = u32;

    fn try_from(index: u32) -> Result<Self, Self::Error> {
        Ok(CellPtr::Ref(Ptr::new(index)))
    }
}

impl From<Ptr<CellPtr>> for Option<CellPtr> {
    fn from(value: Ptr<CellPtr>) -> Self {
        if value.is_nil() {
            return None;
        } else {
            return Some(CellPtr::Ref(value));
        }
    }
}

impl TryFrom<Term> for Cell {
    type Error = Term;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        match value {
            Term::Cell(cell) => Ok(cell),
            _ => Err(value),
        }
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Ctr(_, _) => write!(f, "(Ctr {} {})", 0, 0),
            Cell::Dup(_, _) => write!(f, "(Dup {} {})", 0, 0),
        }
    }
}

pub struct CellPtrDisplay<'a> {
    pub(crate) cell_ptr: &'a CellPtr,
    pub(crate) store: &'a Store,
}
impl<'a> Display for CellPtrDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cell_ptr {
            CellPtr::Era => write!(f, "*"),
            CellPtr::Ref(ptr) => {
                let ports = self.store.read_cell(ptr).unwrap().ports();
                return write!(
                    f,
                    "(Ctr {} {})",
                    self.store.display_term(&ports.0),
                    self.store.display_term(&ports.1)
                );
            }
        }
    }
}
