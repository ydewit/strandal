use std::fmt::Display;

use crate::strandal::store::Store;

use super::{cell::CellPtr, term::TermPtr, var::VarPtr};

// Linear wire
pub struct Port {
    pub(crate) ptr: VarPtr,
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

pub struct EquationDisplay<'a>(pub(crate) &'a Equation, pub(crate) &'a Store);

impl<'a> Display for EquationDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Equation::Redex {
                left_ptr,
                right_ptr,
            } => write!(f, "{}", self.1.display_redex(left_ptr, right_ptr)),
            Equation::Bind { var_ptr, cell_ptr } => {
                write!(f, "{} ↔ {}", var_ptr, self.1.display_cell(cell_ptr))
            }
            Equation::Connect {
                left_ptr,
                right_ptr,
            } => write!(f, "{} ↔ {}", left_ptr, right_ptr),
        }
    }
}
