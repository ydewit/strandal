use std::sync::atomic::AtomicU64;

use super::{heap::Heap, CellPtr, Port, TermPtr, VarPtr};

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Ctr(TermPtr, TermPtr),
    Dup(TermPtr, TermPtr),
}

#[derive(Debug)]
pub struct Var(pub(crate) AtomicU64);
impl Var {
    pub fn new(val: u64) -> Self {
        Var(AtomicU64::new(val))
    }
}

#[derive(Debug)]
pub enum Term {
    Var(Var),
    Cell(Cell),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Equation {
    Redex {
        left_ptr: CellPtr,
        right_ptr: CellPtr,
    },
    Bind {
        var_ptr: VarPtr,
        cell_ptr: CellPtr,
    },
    Connect {
        left_ptr: VarPtr,
        right_ptr: VarPtr,
    },
}

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<VarPtr>,
    pub(crate) body: Vec<Equation>,
    pub(crate) heap: Heap,
}

impl From<Var> for Term {
    fn from(var: Var) -> Self {
        Term::Var(var)
    }
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
}

impl From<Cell> for Term {
    fn from(cell: Cell) -> Self {
        Term::Cell(cell)
    }
}

// pub struct NetBuilder<'a> {
//     net: &mut 'a Net,
// }

// impl<'a> NetBuilder<'a> {
impl Net {
    pub fn var(&mut self) -> (Port, Port) {
        let ptr = self.heap.alloc_var();
        (Port { ptr }, Port { ptr })
    }

    pub fn ctr<T1: Into<TermPtr>, T2: Into<TermPtr>>(&mut self, port_0: T1, port_1: T2) -> CellPtr {
        let ports = Some((port_0, port_1));
        self.heap.alloc_ctr(ports).into()
    }

    pub fn dup<T1: Into<TermPtr>, T2: Into<TermPtr>>(&mut self, port_0: T1, port_1: T2) -> CellPtr {
        self.heap.alloc_dup(Some((port_0, port_1)))
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
}

impl Net {
    pub fn new(capacity: usize) -> Self {
        Net {
            head: Default::default(),
            body: Default::default(),
            heap: Heap::new(capacity),
        }
    }
}

impl TryFrom<Term> for Var {
    type Error = Term;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        match value {
            Term::Var(var) => Ok(var),
            _ => Err(value),
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
