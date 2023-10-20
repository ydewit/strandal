use std::{
    alloc::Layout,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use std::alloc::alloc;
use tracing::warn;

use super::{CellPtr, WirePtr, net::{Term, Wire, Cell}};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct HeapPtr {
    pub(crate) index: u32,
}

#[derive(Debug)]
pub struct Heap {
    mem: *mut Term, // raw mutable pointer
    layout: Layout,
    next: AtomicUsize,
    used: AtomicUsize,
    len: AtomicUsize,
    full: AtomicBool,
}

impl HeapPtr {
    pub fn new(index: u32) -> Self {
        Self { index }
    }
}

impl TryFrom<CellPtr> for HeapPtr {
    type Error = CellPtr;

    fn try_from(value: CellPtr) -> Result<Self, Self::Error> {
        match value {
            CellPtr::Ctr(ptr) => Ok(ptr),
            CellPtr::Dup(ptr) => Ok(ptr),
            CellPtr::Era => Err(value),
        }
    }
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    pub fn new(capacity: usize) -> Self {
        let layout: Layout = Layout::array::<Term>(capacity).expect("Could not allocate heap");
        let mem = unsafe { alloc(layout) } as *mut Term;
        // let mem = NonNull::new(ptr).expect("Could not allocate memory");
        Heap {
            mem,
            layout,
            len: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
            used: AtomicUsize::new(0),
            full: AtomicBool::new(false),
        }
    }

    pub fn len(&self) -> u32 {
        self.len.load(Ordering::SeqCst) as u32
    }

    pub fn alloc_cell(&self) -> CellPtr {
        let index = self._alloc_index(None);
        CellPtr::Ctr(HeapPtr { index })
    }

    pub fn alloc_wire(&self) -> WirePtr {
        let index = self._alloc_index(Some(Term::Wire(Wire::new(u64::MAX))));
        WirePtr(HeapPtr { index })
    }

    pub fn get_cell(&self, ptr: CellPtr) -> Option<Cell> {
        match ptr {
            CellPtr::Era => None,
            CellPtr::Ctr(ptr) | CellPtr::Dup(ptr) => match self._get_term(ptr) {
                Some(Term::Cell(cell)) => Some(*cell),
                Some(Term::Wire(_)) => panic!("Expected cell, found wire"),
                None => None,
            },
        }
    }

    pub fn set_cell(&self, ptr: CellPtr, value: Cell) {
        match ptr {
            CellPtr::Era => panic!("Cannot set ERA"),
            CellPtr::Ctr(_) => {
                assert!(value.is_ctr(), "Cannot set CTR with DUP");
            }
            CellPtr::Dup(ptr) => {
                assert!(value.is_dup(), "Cannot set DUP with CTR");
                self._set_cell(ptr, value);
            }
        }
    }

    pub fn get_wire(&self, wire_ptr: WirePtr) -> Option<CellPtr> {
        let WirePtr(ptr) = wire_ptr;
        match self._get_term(ptr) {
            Some(term) => match term {
                Term::Wire(Wire(atomic)) => {
                    let ptr = atomic.load(Ordering::SeqCst);
                    if ptr == u64::MAX {
                        return None;
                    } else {
                        return Some(ptr.try_into().unwrap());
                    }
                }
                Term::Cell(_) => {
                    panic!("Expected wire, found cell");
                }
            },
            None => todo!(),
        }
    }

    pub fn set_or_get_wire(&self, wire_ptr: WirePtr, value: CellPtr) -> Option<CellPtr> {
        let WirePtr(ptr) = wire_ptr;
        match self._get_term(ptr) {
            Some(term) => match term {
                Term::Wire(Wire(atomic)) => {
                    let new_ptr = value.into();
                    let old_ptr = atomic.swap(new_ptr, Ordering::SeqCst);
                    if old_ptr != u64::MAX {
                        if old_ptr != new_ptr {
                            return Some(old_ptr.try_into().unwrap());
                        } else {
                            warn!("WARN: Setting var with value {:?} twice?", value);
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                Term::Cell(_) => {
                    panic!("Expected wire, found cell");
                }
            },
            None => panic!("Expected wire, found nothing"),
        }
    }

    pub fn free_cell(&self, index: CellPtr) {
        match index {
            CellPtr::Era => panic!("Cannot free unboxed ERA"),
            CellPtr::Ctr(ptr) | CellPtr::Dup(ptr) => {
                self._free(ptr);
            }
        }
    }

    pub fn free_wire(&self, index: WirePtr) {
        self._free(index.0);
    }

    fn _alloc_index(&self, value: Option<Term>) -> u32 {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        self.len.fetch_add(1, Ordering::SeqCst);
        assert!(index < u32::MAX as usize, "heap full");
        // increment total allocated
        if let Some(term) = value {
            unsafe {
                self.mem.add(index).write(term);
            }
        }
        index as u32
    }

    fn _is_null(&self, ptr: HeapPtr) -> bool {
        assert!(ptr.index < self.len());
        let index = ptr.index as usize;
        unsafe { self.mem.add(index).is_null() }
    }

    fn _get_term(&self, ptr: HeapPtr) -> Option<&Term> {
        assert!(ptr.index < self.len());
        let index = ptr.index as usize;
        unsafe {
            if self.mem.add(index).is_null() {
                return None;
            } else {
                return self.mem.add(index).as_ref();
            }
        }
    }

    fn _set_cell(&self, ptr: HeapPtr, value: Cell) {
        assert!(ptr.index < self.len());
        let index = ptr.index as usize;
        unsafe {
            self.mem.add(index).write(Term::Cell(value));
        }
    }

    fn _free(&self, ptr: HeapPtr) {
        assert!(ptr.index < self.len());
        unsafe {
            let mem_ptr = self.mem.add(ptr.index as usize);
            if mem_ptr.is_null() {
                panic!("Cannot free null pointer")
            } else {
                mem_ptr.drop_in_place();
                self.len.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    pub fn iter(&self) -> HeapIter {
        HeapIter {
            index: 0,
            heap: self,
        }
    }
}

pub struct HeapIter<'a> {
    index: usize,
    heap: &'a Heap,
}
impl<'a> Iterator for HeapIter<'a> {
    type Item = &'a Term;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.heap.len() as usize {
            let index = self.index as u32;
            self.index += 1;
            self.heap._get_term(HeapPtr::new(index))
        } else {
            None
        }
    }
}
