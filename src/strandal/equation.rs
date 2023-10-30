use std::fmt::Display;

use crate::strandal::store::Store;

use super::{cell::CellRef, var::VarRef};

#[derive(Debug, Eq, PartialEq)]
pub struct VarPort {
    pub(crate) ptr: VarRef,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Equation {
    Redex {
        left_ref: CellRef,
        right_ref: CellRef,
    },
    Bind {
        var_ref: VarRef,
        cell_ref: CellRef,
    },
    Connect {
        left_ref: VarRef,
        right_ref: VarRef,
    },
}

pub struct EquationDisplay<'a>(pub(crate) &'a Equation, pub(crate) &'a Store);

impl<'a> Display for EquationDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Equation::Redex {
                left_ref,
                right_ref,
            } => write!(f, "{}", self.1.display_redex(left_ref, right_ref)),
            Equation::Bind { var_ref, cell_ref } => {
                write!(f, "{} ↔ {}", var_ref, self.1.display_cell(cell_ref))
            }
            Equation::Connect {
                left_ref,
                right_ref,
            } => write!(f, "{} ↔ {}", left_ref, right_ref),
        }
    }
}
