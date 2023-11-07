use std::fmt::Display;

use super::{
    store::{Ptr, Store},
    term::{Cell, Term, TermPtr},
    var::Var,
};

pub struct VarDisplay<'a>(pub Ptr, pub &'a Var);

impl<'a> Display for VarDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x.{}", self.0.index())
    }
}

pub struct CellPtrDisplay<'a>(pub &'a Store, pub Ptr);
impl<'a> Display for CellPtrDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.get(self.1) {
            Some(Term::Cell(cell)) => CellDisplay(self.0, Some(self.1), cell).fmt(f),
            Some(Term::Var(var)) => VarDisplay(self.1, var).fmt(f),
            None => write!(f, "<n/a>"),
        }
    }
}

pub struct CellDisplay<'a>(pub &'a Store, pub Option<Ptr>, pub &'a Cell);
impl<'a> CellDisplay<'a> {
    pub const ERA_SYMBOL: &'static str = "ε";
    pub const DUP_SYMBOL: &'static str = "δ";
    pub const APP_SYMBOL: &'static str = "@";
    pub const LAM_SYMBOL: &'static str = "λ";
}

impl<'a> Display for CellDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.2 {
            Cell::Dup(ports, lbl) => {
                display_cell(self.0, f, CellDisplay::DUP_SYMBOL, ports, lbl, self.1)
            }

            Cell::App(ports) => {
                display_cell(self.0, f, CellDisplay::APP_SYMBOL, ports, &None, self.1)
            }

            Cell::Lam(ports) => {
                display_cell(self.0, f, CellDisplay::LAM_SYMBOL, ports, &None, self.1)
            }
        }
    }
}

pub struct TermDisplay<'a>(&'a Store, &'a TermPtr);

impl<'a> Display for TermDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            TermPtr::Era => write!(f, "{}", CellDisplay::ERA_SYMBOL),
            TermPtr::Ptr(ptr) => match self.0.get(*ptr) {
                Some(Term::Cell(cell)) => CellDisplay(self.0, Some(*ptr), cell).fmt(f),
                Some(Term::Var(var)) => VarDisplay(*ptr, var).fmt(f),
                None => write!(f, "<n/a>"),
            },
        }
    }
}

fn display_cell<'a>(
    store: &'a Store,
    f: &mut std::fmt::Formatter<'_>,
    symbol: &'static str,
    ports: &Option<(TermPtr, TermPtr)>,
    lbl: &Option<Ptr>,
    ptr: Option<Ptr>,
) -> std::fmt::Result {
    match ports {
        Some((p0, p1)) => match ptr {
            Some(ptr) => match lbl {
                Some(lbl) => write!(
                    f,
                    "({}.{} {} {} {{{}}})",
                    symbol,
                    ptr.index(),
                    TermDisplay(store, p0),
                    TermDisplay(store, p1),
                    lbl.index()
                ),
                None => write!(
                    f,
                    "({}.{} {} {})",
                    symbol,
                    ptr.index(),
                    TermDisplay(store, p0),
                    TermDisplay(store, p1),
                ),
            },
            None => match lbl {
                Some(lbl) => write!(
                    f,
                    "({} {} {} {{{}}})",
                    symbol,
                    TermDisplay(store, p0),
                    TermDisplay(store, p1),
                    lbl.index()
                ),
                None => write!(
                    f,
                    "({} {} {})",
                    symbol,
                    TermDisplay(store, p0),
                    TermDisplay(store, p1),
                ),
            },
        },
        None => match ptr {
            Some(ptr) => write!(f, "(@#{} ⊢ ⊣)", ptr.index()),
            None => write!(f, "(@ ⊢ ⊣)"),
        },
    }
}
