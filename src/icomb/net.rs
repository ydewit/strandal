use std::sync::atomic::AtomicU64;

use super::{TermPtr, CellPtr, WirePtr, heap::Heap};

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Ctr(TermPtr, TermPtr),
    Dup(TermPtr, TermPtr),
}

#[derive(Debug)]
pub struct Wire(pub(crate) AtomicU64);
impl Wire {
    pub fn new(val: u64) -> Self {
        Wire(AtomicU64::new(val))
    }
}

#[derive(Debug)]
pub enum Term {
    Wire(Wire),
    Cell(Cell),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Equation {
    Redex {
        left_ptr: CellPtr,
        right_ptr: CellPtr,
    },
    Bind {
        wire_ptr: WirePtr,
        cell_ptr: CellPtr,
    },
    Connect {
        left_ptr: WirePtr,
        right_ptr: WirePtr,
    },
}

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<WirePtr>,
    pub(crate) body: Vec<Equation>,
    pub(crate) heap: Heap,
}

impl From<Wire> for Term {
    fn from(wire: Wire) -> Self {
        Term::Wire(wire)
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

    pub fn port_0(&self) -> TermPtr {
        match self {
            Cell::Ctr(port_0, _) | Cell::Dup(port_0, _) => *port_0,
        }
    }

    pub fn port_1(&self) -> TermPtr {
        match self {
            Cell::Ctr(port_1, _) | Cell::Dup(port_1, _) => *port_1,
        }
    }
}

impl From<Cell> for Term {
    fn from(cell: Cell) -> Self {
        Term::Cell(cell)
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


impl TryFrom<Term> for Wire {
    type Error = Term;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        match value {
            Term::Wire(wire) => Ok(wire),
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
