//! This module contains types and functions related to variables in the I-Combinator.
//!
//! A variable is a kind of wire that can serve as a buffer for exchanging a cell or another variable
//! between evaluation threads. It is the only value in this implementation that is mutable.
//!
//! A variable value is represented by a `VarValue`, which is a wrapper around an optional term, i.e.
//! a cell or another variable.
//!
//! Thread-safe mutability is guaranteed by using an `AtomicU64` to store the `VarValue`. And making
//! sure that all changes are basically atomic swaps. It is the responsibility of the runtime to make
//! that setting a variable happens in a correct order with respect to other operations.
use std::{fmt::Display, sync::atomic::AtomicU64};

use crate::strandal::store::Ptr;

use super::{
    cell::CellPtr,
    term::{Term, TermPtr},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarPtr(pub(crate) Ptr<VarPtr>);
impl VarPtr {
    pub fn new(index: u32) -> Self {
        VarPtr(Ptr::new(index))
    }
}

impl From<CellPtr> for TermPtr {
    fn from(value: CellPtr) -> Self {
        TermPtr::CellPtr(value)
    }
}

impl From<CellPtr> for u64 {
    fn from(value: CellPtr) -> Self {
        match value {
            CellPtr::Ref(ptr) => (ptr.get_index() as u64) << 1 | false as u64,
            CellPtr::Era => 0,
        }
    }
}
impl From<Ptr<VarPtr>> for Option<VarPtr> {
    fn from(value: Ptr<VarPtr>) -> Self {
        if value.is_nil() {
            return None;
        } else {
            return Some(VarPtr(value));
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct VarValue(Option<TermPtr>);

impl From<VarValue> for u64 {
    fn from(maybe_term_ptr: VarValue) -> Self {
        match maybe_term_ptr.0 {
            None => {
                return 0x0 << 63;
            }
            Some(term_ptr) => {
                let mut value: u64 = 0x1 << 63;
                match term_ptr {
                    TermPtr::CellPtr(cell_ptr) => {
                        value |= 0x0 << 62;
                        match cell_ptr {
                            CellPtr::Ref(ptr) => {
                                return value | 0x0 << 61 | (ptr.get_index() as u64);
                            }
                            CellPtr::Era => {
                                return value | 0x1 << 61;
                            }
                        }
                    }
                    TermPtr::VarPtr(var_ptr) => {
                        value |= 0x1 << 62;
                        return value | (var_ptr.0.get_index() as u64);
                    }
                }
            }
        }
    }
}

impl TryFrom<u64> for VarValue {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value >> 63 == 0 {
            Ok(VarValue(None))
        } else {
            // Some(..)
            if value >> 62 & 0x1 == 0 {
                // CellPtr
                if value >> 61 & 0x1 == 0 {
                    Ok(VarValue(Some(TermPtr::CellPtr(CellPtr::Ref(Ptr::new(
                        (value & 0xFFFFFFFF) as u32,
                    ))))))
                } else {
                    // Era
                    Ok(VarValue(Some(TermPtr::CellPtr(CellPtr::Era))))
                }
            } else {
                // VarPtr
                Ok(VarValue(Some(TermPtr::VarPtr(VarPtr(Ptr::new(
                    (value & 0xFFFFFFFF) as u32,
                ))))))
            }
        }
    }
}

/// A variable is mainly a pointer to a cell that initially is empty and it set once.
/// In addition, a variable may be connected to another variable by setting a pointer
/// to another variable, otherwise it is disconnected (the common case).
///
/// A Var can be in one of four states:
/// 1. Unset & Disconnected: this is the initial state when the Var is created
/// 2. Connected & Unset: this is when the Equation::Connect thread connects the Var to another Var
/// 3. Disconnected & Set: this is when the Equation::Bind thread sets the Var to a CellPtr (this is a final state)
/// 4. Connected & Set: when the var is connected to another var and the other var is set to a CellPtr (this is also a final state)
///
/// Note that the final Var state must always be set. It may or may not be connected depending on whether there is an Equation::Connect.
///
/// The possible orderings of state changes:
/// - 1,2,3,4
/// - 1,3,2,4
/// - 1,3 - this is the common case where the Var is set through a Equation::Bind
///
#[derive(Debug)]
pub struct Var {
    value: AtomicU64, // linked_var_ptr (u32) + cell_ptr (u32)
}
impl Var {
    pub fn new() -> Self {
        Var {
            value: AtomicU64::new(0),
        }
    }

    pub fn set(&self, term_ptr: TermPtr) -> Option<TermPtr> {
        let old_var_value = self.value.swap(
            VarValue(Some(term_ptr)).into(),
            std::sync::atomic::Ordering::Relaxed,
        );
        return VarValue::try_from(old_var_value).unwrap().0;
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

impl Display for VarPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x.{}", self.0.get_index())
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_var_value_cell_ref() {
        let var_value = VarValue(Some(TermPtr::CellPtr(CellPtr::Ref(Ptr::new(1)))));
        let value = u64::from(var_value);
        assert_eq!(VarValue::try_from(value).unwrap(), var_value);
    }

    #[test]
    fn test_var_value_cell_era() {
        let var_value = VarValue(Some(TermPtr::CellPtr(CellPtr::Era)));
        let value = u64::from(var_value);
        assert_eq!(VarValue::try_from(value).unwrap(), var_value);
    }

    #[test]
    fn test_var_value_var_ptr() {
        let var_value = VarValue(Some(TermPtr::VarPtr(VarPtr(Ptr::new(1)))));
        let value = u64::from(var_value);
        assert_eq!(VarValue::try_from(value).unwrap(), var_value);
    }

    #[test]
    fn test_var_value_none() {
        let var_value = VarValue(None);
        let value = u64::from(var_value);
        assert_eq!(VarValue::try_from(value).unwrap(), var_value);
    }
}