use super::store::Store;
use std::fmt::Display;
use tracing::debug;

use super::{
    cell::{Cell, CellRef},
    equation::{Equation, EquationDisplay, VarPort},
    term::TermRef,
    var::VarRef,
};

pub trait NetBuilder {
    fn head<T: Into<TermRef>>(&mut self, term_ref: T);

    fn var(&mut self) -> (VarPort, VarPort);

    fn ctr<T1: Into<TermRef>, T2: Into<TermRef>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellRef;

    fn dup<T1: Into<TermRef>, T2: Into<TermRef>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellRef;

    fn era(&mut self) -> CellRef;

    fn bind(&mut self, wire: VarPort, cell: CellRef);

    fn redex(&mut self, left: CellRef, right: CellRef);

    fn connect(&mut self, left: VarPort, right: VarPort);
}

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<TermRef>,
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

    fn alloc_cell(&self, cell: Option<Cell>) -> CellRef {
        let term = cell.map(|c| c.into());
        let cell_ref = self.store.alloc_cell(term);
        debug!(
            "Allocated store[{}]={}",
            cell_ref.get_ref().unwrap().get_index(),
            self.store.display_cell(&cell_ref)
        );
        cell_ref
    }

    fn alloc_var(&self) -> VarRef {
        let var_ref = self.store.alloc_var();
        debug!("Allocated store[{}]={}", var_ref, var_ref);
        var_ref
    }

    /// Consume the cell identified by the given CellRef. Note that we consume the cell linearly
    fn consume_cell(&self, cell_ref: CellRef) -> Cell {
        match cell_ref {
            CellRef::Era => panic!("Cannot get unboxed ERA from store"),
            CellRef::Ref(ptr) => {
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
    fn head<T: Into<TermRef>>(&mut self, term_ref: T) {
        self.head.push(term_ref.into())
    }

    fn var(&mut self) -> (VarPort, VarPort) {
        let ptr = self.alloc_var();
        (VarPort { ptr }, VarPort { ptr })
    }

    fn ctr<T1: Into<TermRef>, T2: Into<TermRef>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellRef {
        self.alloc_cell(Cell::Ctr(VarPort_0.into(), VarPort_1.into()).into())
            .into()
    }

    fn dup<T1: Into<TermRef>, T2: Into<TermRef>>(
        &mut self,
        VarPort_0: T1,
        VarPort_1: T2,
    ) -> CellRef {
        self.alloc_cell(Cell::Dup(VarPort_0.into(), VarPort_1.into()).into())
            .into()
    }

    fn era(&mut self) -> CellRef {
        CellRef::Era
    }

    fn bind(&mut self, wire: VarPort, cell: CellRef) {
        self.body.push(Equation::Bind {
            var_ref: wire.ptr,
            cell_ref: cell,
        });
    }

    fn redex(&mut self, left: CellRef, right: CellRef) {
        self.body.push(Equation::Redex {
            left_ref: left,
            right_ref: right,
        });
    }

    fn connect(&mut self, left: VarPort, right: VarPort) {
        self.body.push(Equation::Connect {
            left_ref: left.ptr,
            right_ref: right.ptr,
        });
    }

    // fn free<T: Into<TermRef>>(&mut self, term: T) -> VarPort {
    //     let free = self.var();
    //     match term.into() {
    //         TermRef::CellRef(cell_ref) => self.body.push(Equation::Bind {
    //             var_ref: free.0.ptr,
    //             cell_ref,
    //         }),
    //         TermRef::VarRef(var_ref) => self.body.push(Equation::Connect {
    //             left_ref: free.0.ptr,
    //             right_ref: var_ref,
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
        if let Some(term_ref) = head_iter.next() {
            write!(f, "{}", self.0.store.display_term(term_ref))?;
            for head in head_iter {
                write!(f, ", {}", self.0.store.display_term(term_ref))?;
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
