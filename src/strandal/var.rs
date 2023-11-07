use std::sync::atomic::{AtomicU64, Ordering};

use super::store::Ptr;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VarValue {
    Var(Ptr),
    Era,
    Cell(Ptr),
}

impl VarValue {
    fn to_u64(var_value: Option<VarValue>) -> u64 {
        match var_value {
            None => return 0,
            Some(VarValue::Var(ptr)) => return 1 << 62 | ptr.as_u32() as u64,
            Some(VarValue::Era) => return 2 << 62,
            Some(VarValue::Cell(ptr)) => return 3 << 62 | ptr.as_u32() as u64,
        }
    }

    fn from_u64(value: u64) -> Result<Option<VarValue>, u64> {
        let tag = value >> 62;
        if tag == 0 {
            Ok(None)
        } else if tag == 1 {
            Ok(Some(VarValue::Var(Ptr::new((value & 0xFFFFFFFF) as u32))))
        } else if tag == 2 {
            Ok(Some(VarValue::Era))
        } else if tag == 3 {
            Ok(Some(VarValue::Cell(Ptr::new((value & 0xFFFFFFFF) as u32))))
        } else {
            Err(value)
        }
    }
}

#[derive(Debug)]
pub struct Var(AtomicU64);
impl Var {
    pub(crate) fn new() -> Self {
        Var(AtomicU64::new(VarValue::to_u64(None)))
    }

    pub fn set(&self, new_value: VarValue) -> Option<VarValue> {
        let old_value = self.0.swap(
            VarValue::to_u64(Some(new_value)),
            std::sync::atomic::Ordering::Relaxed,
        );
        return VarValue::from_u64(old_value).unwrap();
    }

    pub fn link(&self, var_ptr: Ptr) -> Option<VarValue> {
        return self.set(VarValue::Var(var_ptr));
    }

    pub fn assign_cell(&self, cell_ptr: Ptr) -> Option<VarValue> {
        return self.set(VarValue::Cell(cell_ptr));
    }

    pub fn assign_era(&self) -> Option<VarValue> {
        return self.set(VarValue::Era);
    }

    pub fn read(&self) -> Option<VarValue> {
        let val = self.0.load(std::sync::atomic::Ordering::Relaxed);
        VarValue::from_u64(val).unwrap()
    }
}
impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(Ordering::Relaxed) == other.0.load(Ordering::Relaxed)
    }
}
impl Eq for Var {}

#[derive(Debug, Eq, PartialEq)]
pub struct VarUse {
    ptr: Ptr,
}

impl VarUse {
    pub fn new(ptr: Ptr) -> Self {
        VarUse { ptr }
    }

    pub fn ptr(&self) -> Ptr {
        self.ptr
    }
}
