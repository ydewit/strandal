use std::{sync::atomic::{AtomicUsize, Ordering}, time::Instant};

use tracing::info;

use super::{heap::Heap, CellPtr, WirePtr, TermPtr, net::{Net, Equation, Cell}};

pub struct Runtime {
    anni: AtomicUsize, // anni rewrites
    comm: AtomicUsize, // comm rewrites
    eras: AtomicUsize, // eras rewrites
    redexes: AtomicUsize,
    binds: AtomicUsize,
    connects: AtomicUsize,
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

        // match (heap.get_wire(left_ptr), heap.get_wire(right_ptr)) {
        //     (Some(left_cell_ptr), Some(right_cell_ptr)) => {
        //         self.rewrite_redex(scope, heap, left_cell_ptr, right_cell_ptr)
        //     }
        //     (None, Some(cell_ptr)) => self.eval_bind(scope, heap, left_ptr, cell_ptr),
        //     (Some(cell_ptr), None) => self.eval_bind(scope, heap, right_ptr, cell_ptr),
        //     (None, None) => {
        //         // TODO: See #1
        //     }
        // }
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
