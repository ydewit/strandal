pub mod heap;
pub mod net;
pub mod runtime;

use self::heap::HeapPtr;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TermPtr {
    Cell(CellPtr),
    Wire(WirePtr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellPtr {
    Era,
    Ctr(HeapPtr),
    Dup(HeapPtr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct WirePtr(HeapPtr);

impl From<CellPtr> for TermPtr {
    fn from(value: CellPtr) -> Self {
        TermPtr::Cell(value)
    }
}

impl From<CellPtr> for u64 {
    fn from(value: CellPtr) -> Self {
        match value {
            CellPtr::Ctr(ptr) => (ptr.index as u64) << 1 | false as u64,
            CellPtr::Dup(ptr) => (ptr.index as u64) << 1 | true as u64,
            CellPtr::Era => 0,
        }
    }
}

impl TryFrom<u64> for CellPtr {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let index = value >> 1;
        let is_dup = value & 1 == 1;
        if is_dup {
            Ok(CellPtr::Dup(HeapPtr {
                index: index as u32,
            }))
        } else {
            Ok(CellPtr::Ctr(HeapPtr {
                index: index as u32,
            }))
        }
    }
}

impl From<WirePtr> for TermPtr {
    fn from(value: WirePtr) -> Self {
        TermPtr::Wire(value)
    }
}
