use std::{
    alloc::Layout,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use std::alloc::alloc;
use tracing::{debug, warn};

use super::{
    display::{CellPtrDisplay, TermPtrDisplay},
    net::{Cell, Term, Var},
    CellPtr, TermPtr, VarPtr,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ptr {
    pub(crate) index: u32,
}

#[derive(Debug)]
pub struct Heap {
    mem: *mut Option<Term>, // raw mutable pointer
    // layout: Layout,
    next: AtomicUsize,
    used: AtomicUsize,
    len: AtomicUsize,
    full: AtomicBool,
}

impl Ptr {
    pub fn new(index: u32) -> Self {
        Self { index }
    }
}

impl TryFrom<CellPtr> for Ptr {
    type Error = CellPtr;

    fn try_from(value: CellPtr) -> Result<Self, Self::Error> {
        match value {
            CellPtr::CtrPtr(ptr) => Ok(ptr),
            CellPtr::DupPtr(ptr) => Ok(ptr),
            CellPtr::Era => Err(value),
        }
    }
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}
impl Drop for Heap {
    fn drop(&mut self) {
        unsafe { self.mem.drop_in_place() };
    }
}

impl Heap {
    pub fn new(capacity: usize) -> Self {
        let layout: Layout =
            Layout::array::<Option<Term>>(capacity).expect("Could not allocate heap");
        let mem = unsafe { alloc(layout) } as *mut Option<Term>;
        Heap {
            mem,
            len: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
            used: AtomicUsize::new(0),
            full: AtomicBool::new(false),
        }
    }

    pub fn len(&self) -> u32 {
        self.len.load(Ordering::SeqCst) as u32
    }

    pub fn alloc_ctr<T0: Into<TermPtr>, T1: Into<TermPtr>>(
        &self,
        ports: Option<(T0, T1)>,
    ) -> CellPtr {
        let ctr = match ports {
            Some((port_0, port_1)) => Some(Cell::Ctr(port_0.into(), port_1.into()).into()),
            None => None,
        };
        let index = self._alloc_index(ctr);
        debug!("Allocated CTR[{:?}]", index);
        CellPtr::CtrPtr(Ptr { index })
    }

    pub fn alloc_dup<T0: Into<TermPtr>, T1: Into<TermPtr>>(
        &self,
        ports: Option<(T0, T1)>,
    ) -> CellPtr {
        let dup = match ports {
            Some((port_0, port_1)) => Some(Cell::Dup(port_0.into(), port_1.into()).into()),
            None => None,
        };
        let index = self._alloc_index(dup);
        debug!("Allocated DUP[{:?}]", index);
        CellPtr::CtrPtr(Ptr { index })
    }

    pub fn alloc_var(&self) -> VarPtr {
        let index = self._alloc_index(Some(Var::new(u64::MAX).into()));
        debug!("Allocated VAR[{:?}]", index);
        VarPtr(Ptr { index })
    }

    /// Consume the cell identified by the given CellPtr. Note that we consume the cell linearly
    pub fn consume_cell(&self, ptr: CellPtr) -> (TermPtr, TermPtr) {
        match ptr {
            CellPtr::Era => panic!("Cannot get unboxed ERA"),
            CellPtr::CtrPtr(ptr) | CellPtr::DupPtr(ptr) => {
                debug!("Consumed CELL[{:?}]", ptr.index);
                match self._get_term(ptr) {
                    Some(Term::Cell(Cell::Ctr(port_0, port_1)))
                    | Some(Term::Cell(Cell::Dup(port_0, port_1))) => (port_0, port_1),
                    Some(Term::Var(_)) => panic!("Expected cell, found var"),
                    None => panic!("Expected cell, found nothing"),
                }
            }
        }
    }

    pub(crate) fn read_cell(&self, ptr: CellPtr) -> (TermPtr, TermPtr) {
        match ptr {
            CellPtr::Era => panic!("Cannot get unboxed ERA"),
            CellPtr::CtrPtr(ptr) | CellPtr::DupPtr(ptr) => match self._get_term(ptr) {
                Some(Term::Cell(Cell::Ctr(port_0, port_1)))
                | Some(Term::Cell(Cell::Dup(port_0, port_1))) => (port_0, port_1),
                Some(Term::Var(_)) => panic!("Expected cell, found var"),
                None => panic!("Expected cell, found nothing"),
            },
        }
    }

    pub fn set_cell(&self, cell_ptr: CellPtr, value: Cell) {
        match cell_ptr {
            CellPtr::Era => panic!("Cannot set ERA"),
            CellPtr::CtrPtr(ptr) => {
                assert!(value.is_ctr(), "Cannot set CTR with DUP");
                debug!("Set CTR[{:?}] = {:?}", ptr.index, value);
                self._set_cell(ptr, value);
            }
            CellPtr::DupPtr(ptr) => {
                debug!("Set DUP[{:?}] = {:?}", ptr.index, value);
                assert!(value.is_dup(), "Cannot set DUP with CTR");
                self._set_cell(ptr, value);
            }
        }
    }

    pub fn read_var(&self, var_ptr: VarPtr) -> Option<CellPtr> {
        let VarPtr(ptr) = var_ptr;
        debug!("Read VAR[{:?}]", ptr.index);
        match self._get_term(ptr) {
            Some(term) => match term {
                Term::Var(Var(atomic)) => {
                    let ptr = atomic.load(Ordering::SeqCst);
                    if ptr == u64::MAX {
                        return None;
                    } else {
                        return Some(ptr.try_into().unwrap());
                    }
                }
                Term::Cell(_) => {
                    panic!("Expected var, found cell");
                }
            },
            None => {
                panic!("Expected var, found nothing at {var_ptr:?}");
            }
        }
    }

    pub fn swap_var(&self, var_ptr: VarPtr, value: CellPtr) -> Option<CellPtr> {
        let VarPtr(ptr) = var_ptr;
        debug!("Swap VAR[{:?}] = {:?}", ptr.index, value);
        match self._get_term(ptr) {
            Some(term) => match term {
                Term::Var(Var(atomic)) => {
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
                    panic!("Expected var, found cell");
                }
            },
            None => panic!("Expected var, found nothing"),
        }
    }

    pub fn free_cell(&self, index: CellPtr) {
        match index {
            CellPtr::Era => panic!("Cannot free unboxed ERA"),
            CellPtr::CtrPtr(ptr) => {
                debug!("Free CTR[{:?}]", ptr.index);
                self._free(ptr);
            }
            CellPtr::DupPtr(ptr) => {
                debug!("Free DUP[{:?}]", ptr.index);
                self._free(ptr);
            }
        }
    }

    pub fn free_var(&self, var_ptr: VarPtr) {
        debug!("Free VAR[{:?}]", var_ptr.0);
        self._free(var_ptr.0);
    }

    fn _alloc_index(&self, term: Option<Term>) -> u32 {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        self.len.fetch_add(1, Ordering::SeqCst);
        assert!(index < u32::MAX as usize, "heap full");
        // increment total allocated
        unsafe {
            self.mem.add(index).write(term);
        }
        index as u32
    }

    fn _get_term(&self, ptr: Ptr) -> Option<Term> {
        assert!(ptr.index < self.len());
        let index = ptr.index as usize;
        unsafe { self.mem.add(index).read() }
    }

    fn _set_cell(&self, ptr: Ptr, value: Cell) {
        assert!(ptr.index < self.len());
        let index = ptr.index as usize;
        unsafe {
            self.mem.add(index).write(Some(value.into()));
        }
    }

    fn _free(&self, ptr: Ptr) -> Term {
        assert!(ptr.index < self.len());
        unsafe {
            let item = self.mem.add(ptr.index as usize);
            match item.replace(None) {
                Some(term) => {
                    self.len.fetch_sub(1, Ordering::SeqCst);
                    term
                }
                None => panic!("Cannot free null pointer"),
            }
        }
    }

    pub fn iter(&self) -> HeapIter {
        HeapIter {
            index: 0,
            heap: self,
        }
    }

    pub fn display_term(&self, term_ptr: TermPtr) -> TermPtrDisplay {
        TermPtrDisplay {
            term_ptr,
            heap: self,
        }
    }

    pub fn display_cell(&self, cell_ptr: CellPtr) -> CellPtrDisplay {
        CellPtrDisplay {
            cell_ptr,
            heap: self,
        }
    }

    pub fn display_redex(&self, left_ptr: CellPtr, right_ptr: CellPtr) -> String {
        format!(
            "{} ⋈ {}",
            self.display_cell(left_ptr),
            self.display_cell(right_ptr)
        )
    }

    pub fn display_bind(&self, var_ptr: VarPtr, cell_ptr: CellPtr) -> String {
        format!("{} ← {}", var_ptr, self.display_cell(cell_ptr))
    }

    pub fn display_connect(&self, left_ptr: VarPtr, right_ptr: VarPtr) -> String {
        format!("{} ↔ {}", left_ptr, right_ptr)
    }
}

pub struct HeapIter<'a> {
    index: usize,
    heap: &'a Heap,
}
impl<'a> Iterator for HeapIter<'a> {
    type Item = Term;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.heap.len() as usize {
            let index = self.index as u32;
            self.index += 1;
            self.heap._get_term(Ptr::new(index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::icomb::net::Cell;

    // add tests to the Heap struct here
    use super::{CellPtr, Heap, Ptr, VarPtr};

    #[test]
    fn alloc_cell() {
        let heap = Heap::new(1 << 30);
        let ptr = heap.alloc_ctr((CellPtr::Era, CellPtr::Era).into());
        assert_eq!(ptr, CellPtr::CtrPtr(Ptr::new(0)));
        assert_eq!(
            heap.consume_cell(ptr),
            (CellPtr::Era.into(), CellPtr::Era.into())
        )
    }

    #[test]
    fn alloc_var() {
        let heap = Heap::new(1 << 30);
        let ptr = heap.alloc_var();
        assert_eq!(ptr, VarPtr(Ptr::new(0)));
        assert_eq!(heap.read_var(ptr), None)
    }

    #[test]
    fn alloc_cell_set_ctr() {
        let heap = Heap::new(1 << 30);
        let cell_ptr = heap.alloc_ctr((CellPtr::Era, CellPtr::Era).into());
        heap.set_cell(
            cell_ptr,
            Cell::Ctr(CellPtr::Era.into(), CellPtr::Era.into()),
        );
        assert_eq!(cell_ptr, CellPtr::CtrPtr(Ptr::new(0)));
        assert_eq!(
            heap.consume_cell(cell_ptr),
            (CellPtr::Era.into(), CellPtr::Era.into())
        );
    }

    #[test]
    fn alloc_cell_set_dup() {
        let heap = Heap::new(1 << 30);
        let cell_ptr = heap.alloc_dup((CellPtr::Era, CellPtr::Era).into());
        heap.set_cell(
            cell_ptr,
            Cell::Dup(CellPtr::Era.into(), CellPtr::Era.into()),
        );
        assert_eq!(cell_ptr, CellPtr::DupPtr(Ptr::new(0)));
        assert_eq!(
            heap.consume_cell(cell_ptr),
            (CellPtr::Era.into(), CellPtr::Era.into())
        );
    }
}
