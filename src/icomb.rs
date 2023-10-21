pub mod heap;
pub mod net;
pub mod runtime;

use self::heap::Ptr;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum TermPtr {
    CellPtr(CellPtr),
    VarPtr(VarPtr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellPtr {
    Era,
    CtrPtr(Ptr),
    DupPtr(Ptr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarPtr(Ptr);

impl From<CellPtr> for TermPtr {
    fn from(value: CellPtr) -> Self {
        TermPtr::CellPtr(value)
    }
}

impl From<CellPtr> for u64 {
    fn from(value: CellPtr) -> Self {
        match value {
            CellPtr::CtrPtr(ptr) => (ptr.index as u64) << 1 | false as u64,
            CellPtr::DupPtr(ptr) => (ptr.index as u64) << 1 | true as u64,
            CellPtr::Era => 0,
        }
    }
}

// Linear wire
pub struct Port {
    ptr: VarPtr,
}

impl From<Port> for TermPtr {
    fn from(value: Port) -> Self {
        value.ptr.into()
    }
}

impl TryFrom<u64> for CellPtr {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let index = value >> 1;
        let is_dup = value & 1 == 1;
        if is_dup {
            Ok(CellPtr::DupPtr(Ptr {
                index: index as u32,
            }))
        } else {
            Ok(CellPtr::CtrPtr(Ptr {
                index: index as u32,
            }))
        }
    }
}

impl From<VarPtr> for TermPtr {
    fn from(value: VarPtr) -> Self {
        TermPtr::VarPtr(value)
    }
}
