pub mod cell;
pub mod equation;
pub mod term;
pub mod var;

use std::fmt::Display;

use tracing::debug;

use self::{
    cell::{Cell, CellPtr},
    equation::{Equation, EquationDisplay, Port},
    term::TermPtr,
    var::VarPtr,
};

use super::store::Store;

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<VarPtr>,
    pub(crate) body: Vec<Equation>,
    pub(crate) store: Store,
}

impl Net {
    pub fn new(capacity: u32) -> Self {
        Net {
            head: Default::default(),
            body: Default::default(),
            store: Store::new(capacity),
        }
    }

    pub fn var(&mut self) -> (Port, Port) {
        let ptr = self.alloc_var();
        (Port { ptr }, Port { ptr })
    }

    pub fn ctr<T1: Into<TermPtr>, T2: Into<TermPtr>>(&mut self, port_0: T1, port_1: T2) -> CellPtr {
        self.alloc_cell(Cell::Ctr(port_0.into(), port_1.into()).into())
            .into()
    }

    pub fn dup<T1: Into<TermPtr>, T2: Into<TermPtr>>(&mut self, port_0: T1, port_1: T2) -> CellPtr {
        self.alloc_cell(Cell::Dup(port_0.into(), port_1.into()).into())
            .into()
    }

    pub fn era(&mut self) -> CellPtr {
        CellPtr::Era
    }

    pub fn bind(&mut self, wire: Port, cell: CellPtr) {
        self.body.push(Equation::Bind {
            var_ptr: wire.ptr,
            cell_ptr: cell,
        });
    }

    pub fn redex(&mut self, left: CellPtr, right: CellPtr) {
        self.body.push(Equation::Redex {
            left_ptr: left,
            right_ptr: right,
        });
    }

    pub fn connect(&mut self, left: Port, right: Port) {
        self.body.push(Equation::Connect {
            left_ptr: left.ptr,
            right_ptr: right.ptr,
        });
    }

    pub(crate) fn free<T: Into<TermPtr>>(&mut self, term: T) -> Port {
        let free = self.var();
        match term.into() {
            TermPtr::CellPtr(cell_ptr) => self.body.push(Equation::Bind {
                var_ptr: free.0.ptr,
                cell_ptr,
            }),
            TermPtr::VarPtr(var_ptr) => self.body.push(Equation::Connect {
                left_ptr: free.0.ptr,
                right_ptr: var_ptr,
            }),
        };
        free.0
    }

    //
    fn alloc_cell(&self, cell: Option<Cell>) -> CellPtr {
        let term = cell.map(|c| c.into());
        let cell_ptr = self.store.alloc_cell(term);
        debug!(
            "Allocated store[{}]={}",
            cell_ptr.get_ptr().unwrap().get_index(),
            self.store.display_cell(&cell_ptr)
        );
        cell_ptr
    }

    fn alloc_var(&self) -> VarPtr {
        let var_ptr = self.store.alloc_var();
        debug!("Allocated store[{}]={}", var_ptr, var_ptr);
        var_ptr
    }

    /// Consume the cell identified by the given CellPtr. Note that we consume the cell linearly
    fn consume_cell(&self, cell_ptr: CellPtr) -> Cell {
        match cell_ptr {
            CellPtr::Era => panic!("Cannot get unboxed ERA from store"),
            CellPtr::Ref(ptr) => {
                debug!("Consume CELL[{:?}]", ptr);
                self.store
                    .consume_cell(ptr)
                    .expect("Expected cell, found nothing")
                    .try_into()
                    .unwrap()
            }
        }
    }

    pub fn display_equation<'a>(&'a self, eqn: &'a Equation, store: &'a Store) -> EquationDisplay {
        EquationDisplay(eqn, store)
    }
}

struct NetHead<'a>(&'a Net);
impl Display for NetHead<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = &self.0.head;
        let mut head_iter = head.iter();
        if let Some(head) = head_iter.next() {
            write!(f, "{}", head)?;
            for head in head_iter {
                write!(f, ", {}", head)?;
            }
        }
        return Ok(());
    }
}

struct NetBody<'a>(&'a Net);
impl Display for NetBody<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = &self.0.body;
        let mut body_iter = body.iter();
        if let Some(eqn) = body_iter.next() {
            write!(f, "{}", self.0.display_equation(eqn, &self.0.store))?;
            for eqn in body_iter {
                write!(f, ", {}", self.0.display_equation(eqn, &self.0.store))?;
            }
        }
        return Ok(());
    }
}
impl Display for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "≪ {} | {} ≫", NetHead(self), NetBody(self));
    }
}
