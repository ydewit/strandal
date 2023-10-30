use std::fmt::Display;

use crate::strandal::store::{Ptr, Store};

use super::term::TermRef;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellRef {
    Era,
    Ref(Ptr<CellRef>),
}
impl CellRef {
    #[inline]
    pub fn get_ref(&self) -> Option<&Ptr<CellRef>> {
        match self {
            CellRef::Era => None,
            CellRef::Ref(ptr) => Some(ptr),
        }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Ctr(TermRef, TermRef),
    Dup(TermRef, TermRef),
}

impl Cell {
    pub fn ports(&self) -> (&TermRef, &TermRef) {
        match self {
            Cell::Ctr(port_0, port_1) => (port_0, port_1),
            Cell::Dup(port_0, port_1) => (port_0, port_1),
        }
    }
}

// impl TryFrom<u32> for CellRef {
//     type Error = u32;

//     fn try_from(index: u32) -> Result<Self, Self::Error> {
//         Ok(CellRef::Ref(Ptr::new(index)))
//     }
// }

// impl From<Ptr<CellRef>> for Option<CellRef> {
//     fn from(value: Ptr<CellRef>) -> Self {
//         if value.is_nil() {
//             return None;
//         } else {
//             return Some(CellRef::Ref(value));
//         }
//     }
// }

// impl TryFrom<Term> for Cell {
//     type Error = Term;

//     fn try_from(value: Term) -> Result<Self, Self::Error> {
//         match value {
//             Term::Cell(cell) => Ok(cell),
//             _ => Err(value),
//         }
//     }
// }

// impl Display for Cell {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Cell::Ctr(_, _) => write!(f, "(Ctr {} {})", 0, 0),
//             Cell::Dup(_, _) => write!(f, "(Dup {} {})", 0, 0),
//         }
//     }
// }

pub struct CellRefDisplay<'a> {
    pub(crate) cell_ref: &'a CellRef,
    pub(crate) store: &'a Store,
}
impl<'a> Display for CellRefDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cell_ref {
            CellRef::Era => write!(f, "*"),
            CellRef::Ref(ptr) => {
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
