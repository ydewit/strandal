use std::{
    alloc::Layout,
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};

use std::alloc::alloc;
use tracing::debug;

use super::{
    cell::{Cell, CellRef, CellRefDisplay},
    term::{Term, TermRef, TermRefDisplay},
    var::{Var, VarRef},
};

const NIL_INDEX: u32 = 0;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ptr<T> {
    index: u32,
    _t: PhantomData<T>,
}

impl<T> Ptr<T> {
    pub fn new(index: u32) -> Self {
        assert!(index > 0);
        Self {
            index,
            _t: PhantomData,
        }
    }

    pub fn get_index(&self) -> u32 {
        self.index
    }

    pub fn is_nil(&self) -> bool {
        self.index == NIL_INDEX
    }
}

impl TryFrom<CellRef> for Ptr<CellRef> {
    type Error = CellRef;

    fn try_from(value: CellRef) -> Result<Self, Self::Error> {
        match value {
            CellRef::Ref(ptr) => Ok(ptr),
            CellRef::Era => Err(value),
        }
    }
}

#[derive(Debug)]
pub struct Store {
    mem: *mut Option<Term>, // raw mutable pointer
    next: AtomicUsize,
    used: AtomicUsize,
    capacity: u32,
    full: AtomicBool,
    len: AtomicU32,
}

unsafe impl Send for Store {}
unsafe impl Sync for Store {}

impl Drop for Store {
    fn drop(&mut self) {
        unsafe { self.mem.drop_in_place() };
    }
}

impl Store {
    pub fn new(capacity: u32) -> Self {
        let layout: Layout =
            Layout::array::<Option<Term>>(capacity as usize).expect("Could not allocate Store");
        let mem = unsafe { alloc(layout) } as *mut Option<Term>;
        Store {
            mem,
            capacity,
            next: AtomicUsize::new(1),
            used: AtomicUsize::new(0),
            full: AtomicBool::new(false),
            len: AtomicU32::new(0),
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn len(&self) -> u32 {
        self.len.load(Ordering::Relaxed)
    }

    pub fn alloc_cell(&self, term: Option<Cell>) -> CellRef {
        let index = self.next.fetch_add(1, Ordering::Relaxed);
        assert!(index < u32::MAX as usize, "Store full");
        unsafe {
            self.mem.add(index).write(term.map(|cell| cell.into()));
        }
        self.len.fetch_add(1, Ordering::Relaxed);
        return CellRef::Ref(Ptr::new(index as u32));
    }

    pub fn alloc_var(&self) -> VarRef {
        let index = self.next.fetch_add(1, Ordering::Relaxed);
        assert!(index < u32::MAX as usize, "Store full");
        let var = Var::new();
        unsafe {
            self.mem.add(index).write(Some(var.into()));
        }
        self.len.fetch_add(1, Ordering::Relaxed);
        return VarRef::new(index as u32);
    }

    pub fn consume_cell(&self, ptr: Ptr<CellRef>) -> Option<Cell> {
        unsafe {
            match self._to_mem(ptr.index).read() {
                Some(Term::Cell(cell)) => Some(cell),
                Some(Term::Var(_)) => panic!("Expected cell, found var"),
                None => None,
            }
        }
    }

    pub fn read_cell(&self, ptr: &Ptr<CellRef>) -> Option<&Cell> {
        match unsafe { self._to_mem(ptr.index).as_ref().unwrap() } {
            Some(Term::Cell(cell)) => Some(cell),
            Some(Term::Var(_)) => panic!("Expected cell, found var"),
            None => todo!(),
        }
    }

    pub fn write_cell(&self, ptr: &Ptr<CellRef>, cell: Cell) {
        unsafe { self._to_mem(ptr.index).write(Some(cell.into())) };
    }

    pub fn get_var(&self, ptr: &VarRef) -> &Var {
        match unsafe { self._to_mem(ptr.0.index).as_ref().unwrap() } {
            Some(Term::Var(var)) => var,
            Some(Term::Cell(_)) => panic!("Expected var, found cell"),
            None => panic!("Expected var, found nothing at {ptr:?}"),
        }
    }

    pub fn free_cell(&self, ptr: Ptr<CellRef>) {
        let item = unsafe { self.mem.add(ptr.index as usize) };
        match unsafe { item.replace(None) } {
            Some(_) => {
                self.len.fetch_sub(1, Ordering::Relaxed);
            }
            None => panic!("Cannot free null pointer"),
        }
    }
    pub fn free_var(&self, var_ref: VarRef) {
        let item = unsafe { self.mem.add(var_ref.0.index as usize) };
        match unsafe { item.replace(None) } {
            Some(_) => {
                self.len.fetch_sub(1, Ordering::Relaxed);
                debug!("Free {}", var_ref);
            }
            None => panic!("Cannot free null pointer"),
        }
    }

    #[inline(always)]
    fn _to_mem(&self, index: u32) -> *mut Option<Term> {
        unsafe { self.mem.add(index as usize) }
    }

    #[inline(always)]
    pub fn iter(&self) -> StoreIter {
        StoreIter {
            index: 0,
            store: self,
        }
    }

    pub fn display_term<'a>(&'a self, term_ref: &'a TermRef) -> TermRefDisplay {
        TermRefDisplay {
            term_ref,
            store: self,
        }
    }

    pub fn display_cell<'a>(&'a self, cell_ref: &'a CellRef) -> CellRefDisplay {
        CellRefDisplay {
            cell_ref,
            store: self,
        }
    }

    pub fn display_redex(&self, left_ref: &CellRef, right_ref: &CellRef) -> String {
        format!(
            "{} ⋈ {}",
            self.display_cell(left_ref),
            self.display_cell(right_ref)
        )
    }

    pub fn display_bind(&self, var_ref: &VarRef, cell_ref: &CellRef) -> String {
        format!("{} ← {}", var_ref, self.display_cell(cell_ref))
    }

    pub fn display_connect(&self, left_ref: VarRef, right_ref: VarRef) -> String {
        format!("{} ↔ {}", left_ref, right_ref)
    }
}

pub struct StoreIter<'a> {
    index: u32,
    store: &'a Store,
}
impl<'a> Iterator for StoreIter<'a> {
    type Item = &'a Term;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.store.len() {
            self.index += 1;
            match unsafe { self.store._to_mem(self.index).as_ref().unwrap() } {
                Some(term) => return Some(term),
                None => continue,
            }
        }
        return None;
    }
}

// #[cfg(test)]
// mod tests {

//     use crate::strandal::net::Cell;

//     // add tests to the Store struct here
//     use super::{CellRef, Store, Ptr, VarRef};

//     #[test]
//     fn alloc_cell() {
//         let Store = Store::new(1 << 30);
//         let cell = Cell::Ctr(CellRef::Era.into(), CellRef::Era.into());
//         let ptr = Store.alloc_cell(cell.into());
//         assert_eq!(ptr, CellRef::Ref(Ptr::new(0)));
//         assert_eq!(Store.consume_cell(ptr), cell)
//     }

//     #[test]
//     fn alloc_var() {
//         let Store = Store::new(1 << 30);
//         let ptr = Store.alloc_var();
//         assert_eq!(ptr, VarRef(Ptr::new(0)));
//         assert_eq!(Store.read_var(ptr), None)
//     }

//     #[test]
//     fn alloc_cell_set_ctr() {
//         let Store = Store::new(1 << 30);
//         let cell_ref = Store.alloc_ctr((CellRef::Era, CellRef::Era).into());
//         Store.set_cell(
//             cell_ref,
//             Cell::Ctr(CellRef::Era.into(), CellRef::Era.into()),
//         );
//         assert_eq!(cell_ref, CellRef::CtrPtr(Ptr::new(0)));
//         assert_eq!(
//             Store.consume_cell(cell_ref),
//             (&CellRef::Era.into(), &CellRef::Era.into())
//         );
//     }

//     #[test]
//     fn alloc_cell_set_dup() {
//         let Store = Store::new(1 << 30);
//         let cell_ref = Store.alloc_dup((CellRef::Era, CellRef::Era).into());
//         Store.set_cell(
//             cell_ref,
//             Cell::Dup(CellRef::Era.into(), CellRef::Era.into()),
//         );
//         assert_eq!(cell_ref, CellRef::DupPtr(Ptr::new(0)));
//         assert_eq!(
//             Store.consume_cell(cell_ref),
//             (&CellRef::Era.into(), &CellRef::Era.into())
//         );
//     }
// }
