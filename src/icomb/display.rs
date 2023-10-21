use std::fmt::Display;

use super::{heap::Heap, net::Net, CellPtr, TermPtr, VarPtr};

impl Display for VarPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x.{}", self.0.index)
    }
}

pub struct CellPtrDisplay<'a> {
    pub(crate) cell_ptr: CellPtr,
    pub(crate) heap: &'a Heap,
}
impl<'a> Display for CellPtrDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cell_ptr {
            CellPtr::Era => write!(f, "(era)"),
            CellPtr::CtrPtr(_) => {
                let (port_0, port_1) = self.heap.read_cell(self.cell_ptr);
                return write!(
                    f,
                    "(Ctr {} {})",
                    self.heap.display_term(port_0),
                    self.heap.display_term(port_1)
                );
            }
            CellPtr::DupPtr(_) => {
                let (port_0, port_1) = self.heap.read_cell(self.cell_ptr);
                return write!(
                    f,
                    "(Dup {} {})",
                    self.heap.display_term(port_0),
                    self.heap.display_term(port_1)
                );
            }
        }
    }
}
pub struct TermPtrDisplay<'a> {
    pub(crate) term_ptr: TermPtr,
    pub(crate) heap: &'a Heap,
}
impl<'a> Display for TermPtrDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.term_ptr {
            TermPtr::CellPtr(cell_ptr) => {
                let (port_0, port_1) = self.heap.read_cell(cell_ptr);
                write!(
                    f,
                    "(Ctr {} {}",
                    self.heap.display_term(port_0),
                    self.heap.display_term(port_1)
                )
            }
            TermPtr::VarPtr(var_ptr) => write!(f, "{}", var_ptr),
        }
    }
}
