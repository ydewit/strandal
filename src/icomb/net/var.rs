use std::{fmt::Display, sync::atomic::AtomicU64};

use crate::icomb::store::Ptr;

use super::{
    cell::CellPtr,
    term::{Term, TermPtr},
};

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarPtr(pub(crate) Ptr<VarPtr>);

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

// #[derive(Debug)]
// pub struct VarState {
//     pub(crate) linked_var_ptr: Option<VarPtr>,
//     pub(crate) cell_ptr: Option<CellPtr>,
// }

// impl VarState {
//     pub fn new(linked_var_ptr: Option<VarPtr>, cell_ptr: Option<CellPtr>) -> Self {
//         VarState {
//             linked_var_ptr,
//             cell_ptr,
//         }
//     }
// }
// impl From<u64> for VarState {
//     fn from(value: u64) -> Self {
//         let var_ptr_val: Ptr<VarPtr> = Ptr::new((value >> 32) as u32);
//         let cell_ptr_val: Ptr<CellPtr> = Ptr::new((value & 0xFFFFFFFF) as u32);
//         return VarState::new(var_ptr_val.into(), cell_ptr_val.into());
//     }
// }

// impl From<&VarState> for u64 {
//     fn from(value: &VarState) -> Self {
//         let var_ptr_val = value
//             .linked_var_ptr
//             .map_or(0, |var_ptr| var_ptr.0.index as u64)
//             << 32;
//         let cell_ptr_val = 0xFFFFFFFF
//             & value.cell_ptr.map_or(0, |cell_ptr| match cell_ptr {
//                 CellPtr::Era => panic!("Cannot set ERA"),
//                 CellPtr::Ref(ptr) => ptr.index as u64,
//             });
//         return var_ptr_val | cell_ptr_val;
//     }
// }

// impl From<VarState> for u64 {
//     fn from(value: VarState) -> Self {
//         let var_ptr_val = value
//             .linked_var_ptr
//             .map_or(0, |var_ptr| var_ptr.0.index as u64)
//             << 32;
//         let cell_ptr_val = 0xFFFFFFFF
//             & value.cell_ptr.map_or(0, |cell_ptr| match cell_ptr {
//                 CellPtr::Era => panic!("Cannot set ERA"),
//                 CellPtr::Ref(ptr) => ptr.index as u64,
//             });
//         return var_ptr_val | cell_ptr_val;
//     }
// }

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

    /// Before a Var can be updated it has to be read first to make sure we can compare and swap the write
    ///
    // #[inline(always)]
    // pub fn current_state(&self) -> VarState {
    //     let value = self.value.load(std::sync::atomic::Ordering::Relaxed);
    //     return value.into();
    // }

    pub fn set(&self, term_ptr: TermPtr) -> Option<TermPtr> {
        let old_var_value = self.value.swap(
            VarValue(Some(term_ptr)).into(),
            std::sync::atomic::Ordering::Relaxed,
        );
        return VarValue::try_from(old_var_value).unwrap().0;
    }

    // /// 1. Get the state for the var with current_state()
    // /// 2. connect(). If it returns Some(state) then the var was modified betwee the read and this write. None, if successul.
    // pub fn link(&self, state: &VarState, var_ptr: VarPtr) -> Option<VarState> {
    //     assert!(
    //         state.linked_var_ptr.is_none(),
    //         "Var is already connected to var: {:?}",
    //         state.linked_var_ptr
    //     );

    //     match self.value.compare_exchange(
    //         state.into(),
    //         VarState::new(Some(var_ptr), state.cell_ptr).into(),
    //         std::sync::atomic::Ordering::Relaxed,
    //         std::sync::atomic::Ordering::Relaxed,
    //     ) {
    //         Ok(_) => return None,
    //         Err(current) => Some(current.into()),
    //     }
    // }

    // /// 1. Get the state for the var with current_state()
    // /// 2. bind(). If it returns Some(state) then the var was modified betwee the read and this write. None, if successul.
    // pub fn bind(&self, state: &VarState, cell_ptr: CellPtr) -> Option<VarState> {
    //     assert!(
    //         state.cell_ptr.is_none(),
    //         "Var is already bound to a CellPtr: {:?}",
    //         state.cell_ptr
    //     );

    //     match self.value.compare_exchange(
    //         state.into(),
    //         VarState::new(state.linked_var_ptr, Some(cell_ptr)).into(),
    //         std::sync::atomic::Ordering::Relaxed,
    //         std::sync::atomic::Ordering::Relaxed,
    //     ) {
    //         Ok(_) => return None,
    //         Err(current) => return Some(current.into()),
    //     }
    // }
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
