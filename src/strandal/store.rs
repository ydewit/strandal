use std::{
    alloc::{alloc, Layout},
    fmt::Display,
    sync::atomic::{AtomicU32, Ordering},
};

use super::term::Term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ptr(u32);
impl Ptr {
    #[inline]
    pub fn new(value: u32) -> Self {
        Ptr(value)
    }

    #[inline]
    pub fn index(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

impl Display for Ptr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug)]
pub struct Store {
    mem: *mut Option<Term>, // raw mutable pointer
    pub capacity: u32,
    next: AtomicU32,
    len: AtomicU32,
}

impl Drop for Store {
    fn drop(&mut self) {
        let layout: Layout = Layout::array::<Option<Term>>(self.capacity as usize)
            .expect("Could not deallocate Store");
        unsafe {
            std::alloc::dealloc(self.mem as *mut u8, layout);
        }
    }
}
impl Store {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(2 << 24)
    }

    pub fn with_capacity(capacity: u32) -> Self {
        let layout: Layout =
            Layout::array::<Option<Term>>(capacity as usize).expect("Could not allocate Store");
        let mem = unsafe { alloc(layout) } as *mut Option<Term>;
        assert!(!mem.is_null(), "Could not allocate Store");
        Store {
            mem,
            capacity,
            next: AtomicU32::new(0),
            len: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> u32 {
        return self.len.load(Ordering::Relaxed);
    }

    #[inline]
    pub fn next(&self) -> u32 {
        return self.next.load(Ordering::Relaxed);
    }

    #[inline]
    pub fn alloc(&self, value: Option<Term>) -> Ptr {
        let ptr = self.inc_next();
        unsafe {
            self.ptr(ptr).write(value);
            self.len.fetch_add(1, Ordering::Relaxed);
            return ptr;
        }
    }
    #[inline]
    pub fn free(&self, ptr: Ptr) -> Option<Term> {
        unsafe {
            self.len.fetch_sub(1, Ordering::Relaxed);
            return self.ptr(ptr).replace(None);
        }
    }

    #[inline]
    pub fn get(&self, ptr: Ptr) -> &Option<Term> {
        unsafe {
            return self.ptr(ptr).as_ref().expect("Index out of bounds");
        }
    }

    #[inline]
    pub fn set(&self, ptr: Ptr, term: Term) -> Option<Term> {
        unsafe {
            return self.ptr(ptr).replace(Some(term));
        }
    }

    #[inline]
    unsafe fn ptr(&self, index: Ptr) -> *mut Option<Term> {
        self.mem.add(index.0 as usize)
    }

    #[inline]
    fn inc_next(&self) -> Ptr {
        return Ptr(self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    }
}

unsafe impl Send for Store {}
unsafe impl Sync for Store {}

const FREE_PTRS_SIZE: usize = 20;
pub struct FreePtrs<const N: usize = FREE_PTRS_SIZE> {
    ptrs: [Option<Ptr>; N],
    len: usize,
}
impl<const N: usize> FreePtrs<N> {
    #[inline]
    pub fn new() -> Self {
        FreePtrs {
            ptrs: [None; N],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, ptr: Ptr) {
        self.ptrs[self.len] = Some(ptr);
        self.len += 1;
    }

    #[inline]
    pub fn pop(&mut self) -> Option<Ptr> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            self.ptrs[self.len]
        }
    }

    pub fn split(&mut self, n: u8) -> FreePtrs {
        let split = self.len / n as usize;
        let mut new = FreePtrs::new();
        new.len = self.len - split;
        for i in 0..new.len {
            new.ptrs[i] = self.ptrs[split + i];
            self.ptrs[split + i] = None;
        }
        self.len = split;
        new
    }
}

#[cfg(test)]
mod tests {
    use crate::strandal::{store::Store, term::Term, var::Var};

    #[test]
    fn test_alloc() {
        let store = Store::new();
        let ptr = store.alloc(Some(Term::Var(Var::new())));
        assert_eq!(ptr.index(), 0);
        assert_eq!(store.len(), 1);
        assert_eq!(store.next(), 1);
        assert_eq!(store.get(ptr), &Some(Term::Var(Var::new())));
        assert_eq!(store.free(ptr), Some(Term::Var(Var::new())));
        assert_eq!(store.len(), 0);
        assert_eq!(store.next(), 1);
        assert_eq!(store.get(ptr), &None);
    }
}
