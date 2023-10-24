use super::store::Store;
use std::fmt::Display;
use tracing::debug;

use super::{
    cell::{Cell, CellPtr},
    equation::{Equation, EquationDisplay, VarPort},
    term::TermPtr,
    var::VarPtr,
};

pub trait NetBuilder {
    fn head<T: Into<TermPtr>>(&mut self, term_ptr: T);

    fn var(&mut self) -> (VarPort, VarPort);

    fn ctr<T1: Into<TermPtr>, T2: Into<TermPtr>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellPtr;

    fn dup<T1: Into<TermPtr>, T2: Into<TermPtr>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellPtr;

    fn era(&mut self) -> CellPtr;

    fn bind(&mut self, wire: VarPort, cell: CellPtr);

    fn redex(&mut self, left: CellPtr, right: CellPtr);

    fn connect(&mut self, left: VarPort, right: VarPort);
}

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<TermPtr>,
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

impl NetBuilder for Net {
    fn head<T: Into<TermPtr>>(&mut self, term_ptr: T) {
        self.head.push(term_ptr.into())
    }

    fn var(&mut self) -> (VarPort, VarPort) {
        let ptr = self.alloc_var();
        (VarPort { ptr }, VarPort { ptr })
    }

    fn ctr<T1: Into<TermPtr>, T2: Into<TermPtr>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellPtr {
        self.alloc_cell(Cell::Ctr(VarPort_0.into(), VarPort_1.into()).into())
            .into()
    }

    fn dup<T1: Into<TermPtr>, T2: Into<TermPtr>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellPtr {
        self.alloc_cell(Cell::Dup(VarPort_0.into(), VarPort_1.into()).into())
            .into()
    }

    fn era(&mut self) -> CellPtr {
        CellPtr::Era
    }

    fn bind(&mut self, wire: VarPort, cell: CellPtr) {
        self.body.push(Equation::Bind {
            var_ptr: wire.ptr,
            cell_ptr: cell,
        });
    }

    fn redex(&mut self, left: CellPtr, right: CellPtr) {
        self.body.push(Equation::Redex {
            left_ptr: left,
            right_ptr: right,
        });
    }

    fn connect(&mut self, left: VarPort, right: VarPort) {
        self.body.push(Equation::Connect {
            left_ptr: left.ptr,
            right_ptr: right.ptr,
        });
    }

    // fn free<T: Into<TermPtr>>(&mut self, term: T) -> VarPort {
    //     let free = self.var();
    //     match term.into() {
    //         TermPtr::CellPtr(cell_ptr) => self.body.push(Equation::Bind {
    //             var_ptr: free.0.ptr,
    //             cell_ptr,
    //         }),
    //         TermPtr::VarPtr(var_ptr) => self.body.push(Equation::Connect {
    //             left_ptr: free.0.ptr,
    //             right_ptr: var_ptr,
    //         }),
    //     };
    //     free.0
    // }
}
struct NetHead<'a>(&'a Net);
impl Display for NetHead<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = &self.0.head;
        let mut head_iter = head.iter();
        if let Some(term_ptr) = head_iter.next() {
            write!(f, "{}", self.0.store.display_term(term_ptr))?;
            for head in head_iter {
                write!(f, ", {}", self.0.store.display_term(term_ptr))?;
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
