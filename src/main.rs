use core::panic;
use std::{
    alloc::Layout,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}, time::Instant,
};

use std::alloc::alloc;

use tracing::{info, warn};

fn id(net: &mut Net) -> WirePtr {
    let root_wire = net.heap.alloc_wire();
    let id_wire = net.heap.alloc_wire();
    let lam_ptr = net.heap.alloc_cell();
    net.heap
        .set_cell(lam_ptr, Cell::Ctr(id_wire.into(), id_wire.into()));
    net.body.push(Equation::Bind {
        wire_ptr: root_wire,
        cell_ptr: lam_ptr,
    });
    return root_wire;
}

fn dup(net: &mut Net) -> WirePtr {
    let root_wire = net.heap.alloc_wire();

    let lam_ptr = net.heap.alloc_cell();
    let dup_ptr = net.heap.alloc_cell();
    let app_ptr = net.heap.alloc_cell();

    let wire1 = net.heap.alloc_wire();
    let wire2 = net.heap.alloc_wire();

    net.heap
        .set_cell(lam_ptr, Cell::Ctr(dup_ptr.into(), wire1.into()));
    net.heap
        .set_cell(dup_ptr, Cell::Ctr(wire2.into(), app_ptr.into()));
    net.heap
        .set_cell(app_ptr, Cell::Ctr(wire2.into(), wire1.into()));

    net.body.push(Equation::Bind {
        wire_ptr: root_wire,
        cell_ptr: lam_ptr,
    });
    return root_wire;
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut net = Net::new(1 << 30);
    let id_ptr = id(&mut net);
    let dup_ptr = dup(&mut net);

    net.body.push(Equation::Connect {
        left_ptr: id_ptr,
        right_ptr: dup_ptr,
    });

    info!("Initial Net: {:?}", net);

    let mut runtime = Runtime::new();
    runtime.eval(&mut net);

    info!("Final Net: {:?}", net);
    info!("Redexes: {}", runtime.redexes());
    info!("Binds: {}", runtime.binds());
    info!("Connects: {}", runtime.connects());
    info!("Annihilations: {}", runtime.annihilations());
    info!("Commutations: {}", runtime.commutations());
    info!("Erasures: {}", runtime.erasures());
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum TermPtr {
    Cell(CellPtr),
    Wire(WirePtr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum CellPtr {
    Era,
    Ctr(HeapPtr),
    Dup(HeapPtr),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct WirePtr(HeapPtr);

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Cell {
    Ctr(TermPtr, TermPtr),
    Dup(TermPtr, TermPtr),
}

#[derive(Debug)]
struct Wire(AtomicU64);
impl Wire {
    pub fn new(val: u64) -> Self {
        Wire(AtomicU64::new(val))
    }
}

#[derive(Debug)]
enum Term {
    Wire(Wire),
    Cell(Cell),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Equation {
    Redex {
        left_ptr: CellPtr,
        right_ptr: CellPtr,
    },
    Bind {
        wire_ptr: WirePtr,
        cell_ptr: CellPtr,
    },
    Connect {
        left_ptr: WirePtr,
        right_ptr: WirePtr,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct HeapPtr {
    index: u32,
}

#[derive(Debug)]
struct Heap {
    mem: *mut Term, // raw mutable pointer
    layout: Layout,
    next: AtomicUsize,
    used: AtomicUsize,
    len: AtomicUsize,
    full: AtomicBool,
}

#[derive(Debug)]
struct Net {
    head: Vec<WirePtr>,
    body: Vec<Equation>,
    heap: Heap,
}

struct Runtime {
    anni: AtomicUsize, // anni rewrites
    comm: AtomicUsize, // comm rewrites
    eras: AtomicUsize, // eras rewrites
    redexes: AtomicUsize,
    binds: AtomicUsize,
    connects: AtomicUsize,
}

impl From<Wire> for Term {
    fn from(wire: Wire) -> Self {
        Term::Wire(wire)
    }
}

impl Cell {
    pub fn is_ctr(&self) -> bool {
        match self {
            Cell::Ctr(_, _) => true,
            _ => false,
        }
    }

    pub fn is_dup(&self) -> bool {
        match self {
            Cell::Dup(_, _) => true,
            _ => false,
        }
    }

    fn port_0(&self) -> TermPtr {
        match self {
            Cell::Ctr(port_0, _) | Cell::Dup(port_0, _) => *port_0,
        }
    }

    fn port_1(&self) -> TermPtr {
        match self {
            Cell::Ctr(port_1, _) | Cell::Dup(port_1, _) => *port_1,
        }
    }
}

impl From<Cell> for Term {
    fn from(cell: Cell) -> Self {
        Term::Cell(cell)
    }
}

impl Net {
    pub fn new(capacity: usize) -> Self {
        Net {
            head: Default::default(),
            body: Default::default(),
            heap: Heap::new(capacity),
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            anni: AtomicUsize::new(0),
            comm: AtomicUsize::new(0),
            eras: AtomicUsize::new(0),
            redexes: AtomicUsize::new(0),
            binds: AtomicUsize::new(0),
            connects: AtomicUsize::new(0),
        }
    }

    pub fn annihilations(&self) -> usize {
        self.anni.load(Ordering::SeqCst)
    }

    fn inc_annihilations(&self) {
        self.anni.fetch_add(1, Ordering::SeqCst);
    }

    pub fn commutations(&self) -> usize {
        self.comm.load(Ordering::SeqCst)
    }

    fn inc_comm(&self) {
        self.comm.fetch_add(1, Ordering::SeqCst);
    }

    pub fn erasures(&self) -> usize {
        self.eras.load(Ordering::SeqCst)
    }

    fn inc_erasures(&self) {
        self.eras.fetch_add(1, Ordering::SeqCst);
    }

    pub fn redexes(&self) -> usize {
        self.redexes.load(Ordering::SeqCst)
    }

    fn inc_redexes(&self) {
        self.redexes.fetch_add(1, Ordering::SeqCst);
    }

    pub fn binds(&self) -> usize {
        self.binds.load(Ordering::SeqCst)
    }

    pub fn inc_binds(&self) {
        self.binds.fetch_add(1, Ordering::SeqCst);
    }
    pub fn connects(&self) -> usize {
        self.connects.load(Ordering::SeqCst)
    }

    pub fn inc_connects(&self) {
        self.connects.fetch_add(1, Ordering::SeqCst);
    }
    pub fn eval(&mut self, net: &mut Net) {
        let now = Instant::now();
        rayon::scope(|scope| {
            net.body
                .drain(..)
                .for_each(|eqn| self.eval_equation(scope, &net.heap, eqn));
        });
        info!("Net evaluated in {}", now.elapsed().as_millis());
    }

    fn eval_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        heap: &'scope Heap,
        eqn: Equation,
    ) {
        match eqn {
            Equation::Redex {
                left_ptr,
                right_ptr,
            } => self.eval_redex(scope, heap, left_ptr, right_ptr),
            Equation::Bind { wire_ptr, cell_ptr } => {
                self.eval_bind(scope, heap, wire_ptr, cell_ptr)
            }
            Equation::Connect {
                left_ptr,
                right_ptr,
            } => self.eval_connect(scope, heap, left_ptr, right_ptr),
        }
    }

    #[inline]
    fn rewrite_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        heap: &'scope Heap,
        left_ptr: CellPtr,
        right_ptr: CellPtr,
    ) {
        scope.spawn(move |scope| self.eval_redex(scope, heap, left_ptr, right_ptr));
    }

    fn eval_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        heap: &'scope Heap,
        left_cell_ptr: CellPtr,
        right_cell_ptr: CellPtr,
    ) {
        self.inc_redexes();

        // eval unboxed ERA
        match (left_cell_ptr, right_cell_ptr) {
            // Annihilate: ERA-ERA
            (CellPtr::Era, CellPtr::Era) => {
                // PUFF! Do nothing
                self.inc_erasures();
                // nothing free
            }
            // Annihilate: CTR-CTR / DUP-DUP
            (CellPtr::Ctr(_), CellPtr::Ctr(_)) | (CellPtr::Dup(_), CellPtr::Dup(_)) => {
                // stats
                self.inc_annihilations();

                let left_cell: Cell = heap.get_cell(left_cell_ptr).unwrap();
                let right_cell: Cell = heap.get_cell(left_cell_ptr).unwrap();

                let eqn_0 = Self::to_equation(left_cell.port_0(), right_cell.port_0());
                self.eval_equation(scope, heap, eqn_0);

                let eqn_1 = Self::to_equation(left_cell.port_1(), right_cell.port_1());
                self.eval_equation(scope, heap, eqn_1);

                // TODO reuse?
                heap.free_cell(left_cell_ptr); // TODO: Can we reuse this?
                heap.free_cell(right_cell_ptr); // TODO can we reuse this?
            }
            // Commute: ERA-DUP / ERA-CTR
            (CellPtr::Era, cell_ptr @ CellPtr::Ctr(_))
            | (cell_ptr @ CellPtr::Ctr(_), CellPtr::Era)
            | (CellPtr::Era, cell_ptr @ CellPtr::Dup(_))
            | (cell_ptr @ CellPtr::Dup(_), CellPtr::Era) => {
                // stats
                self.inc_comm();

                //
                let ctr_cell: Cell = heap.get_cell(cell_ptr).unwrap();
                let eqn_0 = Self::to_equation(CellPtr::Era.into(), ctr_cell.port_0());
                self.eval_equation(scope, heap, eqn_0);
                let eqn_1 = Self::to_equation(CellPtr::Era.into(), ctr_cell.port_1());
                self.eval_equation(scope, heap, eqn_1);

                // TODO reuse
                heap.free_cell(cell_ptr); // TODO: can we reuse this?
            }
            // Commute: CTR-DUP
            (ctr_ptr @ CellPtr::Ctr(_), dup_ptr @ CellPtr::Dup(_))
            | (dup_ptr @ CellPtr::Dup(_), ctr_ptr @ CellPtr::Ctr(_)) => {
                // stats
                self.inc_comm();

                let ctr: Cell = heap.get_cell(ctr_ptr).unwrap();
                let dup: Cell = heap.get_cell(dup_ptr).unwrap();

                let ctr_ptr_2 = heap.alloc_cell();
                let dup_ptr_2 = heap.alloc_cell();

                let eqn1 = Self::to_equation(ctr.port_0(), dup_ptr.into());
                let eqn2 = Self::to_equation(ctr.port_1(), dup_ptr_2.into());
                let eqn3 = Self::to_equation(dup.port_0(), ctr_ptr.into());
                let eqn4 = Self::to_equation(dup.port_1(), ctr_ptr_2.into());

                let wire_ptr_1 = heap.alloc_wire();
                let wire_ptr_2 = heap.alloc_wire();
                let wire_ptr_3 = heap.alloc_wire();
                let wire_ptr_4 = heap.alloc_wire();

                let ctr_1 = Cell::Ctr(wire_ptr_1.into(), wire_ptr_2.into());
                let dup_1 = Cell::Dup(wire_ptr_1.into(), wire_ptr_2.into());
                let ctr_2: Cell = Cell::Ctr(wire_ptr_3.into(), wire_ptr_4.into());
                let dup_2 = Cell::Dup(wire_ptr_3.into(), wire_ptr_4.into());

                heap.set_cell(ctr_ptr, ctr_1); // NOTE: ctr_ptr is reused here!
                heap.set_cell(ctr_ptr_2, ctr_2);
                heap.set_cell(dup_ptr, dup_1); // NOTE: dup_ctr is reused here!
                heap.set_cell(dup_ptr_2, dup_2);

                self.eval_equation(scope, heap, eqn1);
                self.eval_equation(scope, heap, eqn2);
                self.eval_equation(scope, heap, eqn3);
                self.eval_equation(scope, heap, eqn4);
            }
        }
    }

    fn eval_bind<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        heap: &'scope Heap,
        wire_ptr: WirePtr,
        cell_ptr: CellPtr,
    ) {
        self.inc_binds();

        match heap.set_or_get_wire(wire_ptr, cell_ptr) {
            Some(other_cell_ptr) => {
                self.rewrite_redex(scope, heap, cell_ptr, other_cell_ptr);
            }
            None => {
                // value set for the first time
            }
        }
    }

    fn eval_connect<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        heap: &'scope Heap,
        left_ptr: WirePtr,
        right_ptr: WirePtr,
    ) {
        self.inc_connects();
        // let left_wire = heap.get(left_ptr.into());
        // let right_wire = heap.get(right_ptr.into());
    }

    fn to_equation(left_ptr: TermPtr, right_ptr: TermPtr) -> Equation {
        match (left_ptr, right_ptr) {
            (TermPtr::Cell(left_ptr), TermPtr::Cell(right_ptr)) => Equation::Redex {
                left_ptr,
                right_ptr,
            },
            (TermPtr::Cell(cell_ptr), TermPtr::Wire(wire_ptr))
            | (TermPtr::Wire(wire_ptr), TermPtr::Cell(cell_ptr)) => {
                Equation::Bind { wire_ptr, cell_ptr }
            }
            (TermPtr::Wire(left_ptr), TermPtr::Wire(right_ptr)) => Equation::Connect {
                left_ptr,
                right_ptr,
            },
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
impl TryFrom<Term> for Wire {
    type Error = Term;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        match value {
            Term::Wire(wire) => Ok(wire),
            _ => Err(value),
        }
    }
}

impl TryFrom<Term> for Cell {
    type Error = Term;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        match value {
            Term::Cell(cell) => Ok(cell),
            _ => Err(value),
        }
    }
}
