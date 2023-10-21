use std::{fmt::Display, sync::atomic::AtomicU64};

use super::{
    display::{CellPtrDisplay, TermPtrDisplay},
    heap::Heap,
    CellPtr, Port, TermPtr, VarPtr,
};

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Ctr(TermPtr, TermPtr),
    Dup(TermPtr, TermPtr),
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Ctr(_, _) => write!(f, "(Ctr {} {})", 0, 0),
            Cell::Dup(_, _) => write!(f, "(Dup {} {})", 0, 0),
        }
    }
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
            write!(f, "{}", EquationDisplay(eqn, &self.0.heap))?;
            for eqn in body_iter {
                write!(f, ", {}", EquationDisplay(eqn, &self.0.heap))?;
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

pub struct EquationDisplay<'a>(&'a Equation, &'a Heap);

impl<'a> Display for EquationDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Equation::Redex {
                left_ptr,
                right_ptr,
            } => write!(f, "{}", self.1.display_redex(*left_ptr, *right_ptr)),
            Equation::Bind { var_ptr, cell_ptr } => {
                write!(f, "{} ↔ {}", var_ptr, self.1.display_cell(*cell_ptr))
            }
            Equation::Connect {
                left_ptr,
                right_ptr,
            } => write!(f, "{} ↔ {}", left_ptr, right_ptr),
        }
    }
}
