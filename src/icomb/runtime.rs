use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use tracing::info;

use super::{
    heap::Heap,
    net::{Cell, Equation, Net},
    CellPtr, TermPtr, VarPtr,
};

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

    #[inline(always)]
    pub fn annihilations(&self) -> usize {
        self.anni.load(Ordering::SeqCst)
    }

    #[inline(always)]
    fn inc_annihilations(&self) {
        self.anni.fetch_add(1, Ordering::SeqCst);
    }

    pub fn commutations(&self) -> usize {
        self.comm.load(Ordering::SeqCst)
    }

    fn inc_comm(&self) {
        self.comm.fetch_add(1, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn erasures(&self) -> usize {
        self.eras.load(Ordering::SeqCst)
    }

    #[inline(always)]
    fn inc_erasures(&self) {
        self.eras.fetch_add(1, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn redexes(&self) -> usize {
        self.redexes.load(Ordering::SeqCst)
    }

    #[inline(always)]
    fn inc_redexes(&self) {
        self.redexes.fetch_add(1, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn binds(&self) -> usize {
        self.binds.load(Ordering::SeqCst)
    }

    #[inline(always)]
    pub fn inc_binds(&self) {
        self.binds.fetch_add(1, Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn connects(&self) -> usize {
        self.connects.load(Ordering::SeqCst)
    }

    #[inline(always)]
    pub fn inc_connects(&self) {
        self.connects.fetch_add(1, Ordering::SeqCst);
    }

    #[inline(always)]
    fn thread_id(&self) -> usize {
        return rayon::current_thread_index().unwrap();
    }

    pub fn eval(&mut self, net: &mut Net) {
        let now = Instant::now();
        rayon::scope(|scope| {
            net.body
                .drain(..)
                .for_each(|eqn| self.eval_equation(scope, &net.heap, eqn));
        });
        info!("Net evaluated in {:0.00}", now.elapsed().as_nanos() / 1000);
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
            Equation::Bind { var_ptr, cell_ptr } => self.eval_bind(scope, heap, var_ptr, cell_ptr),
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
            (CellPtr::CtrPtr(_), CellPtr::CtrPtr(_)) | (CellPtr::DupPtr(_), CellPtr::DupPtr(_)) => {
                // stats
                self.inc_annihilations();

                let left_cell = heap.consume_cell(left_cell_ptr).unwrap();
                let right_cell = heap.consume_cell(left_cell_ptr).unwrap();

                let eqn_0 = Self::to_equation(left_cell.0, right_cell.0);
                self.eval_equation(scope, heap, eqn_0);

                let eqn_1 = Self::to_equation(left_cell.1, right_cell.1);
                self.eval_equation(scope, heap, eqn_1);

                // TODO reuse?
                heap.free_cell(left_cell_ptr); // TODO: Can we reuse this?
                heap.free_cell(right_cell_ptr); // TODO can we reuse this?
            }
            // Commute: ERA-DUP / ERA-CTR
            (CellPtr::Era, cell_ptr @ CellPtr::CtrPtr(_))
            | (cell_ptr @ CellPtr::CtrPtr(_), CellPtr::Era)
            | (CellPtr::Era, cell_ptr @ CellPtr::DupPtr(_))
            | (cell_ptr @ CellPtr::DupPtr(_), CellPtr::Era) => {
                // stats
                self.inc_comm();

                //
                let ctr_cell = heap.consume_cell(cell_ptr).unwrap();
                let eqn_0 = Self::to_equation(CellPtr::Era.into(), ctr_cell.0);
                self.eval_equation(scope, heap, eqn_0);
                let eqn_1 = Self::to_equation(CellPtr::Era.into(), ctr_cell.1);
                self.eval_equation(scope, heap, eqn_1);

                // TODO reuse
                heap.free_cell(cell_ptr); // TODO: can we reuse this?
            }
            // Commute: CTR-DUP
            (ctr_ptr @ CellPtr::CtrPtr(_), dup_ptr @ CellPtr::DupPtr(_))
            | (dup_ptr @ CellPtr::DupPtr(_), ctr_ptr @ CellPtr::CtrPtr(_)) => {
                // stats
                self.inc_comm();

                let ctr = heap.consume_cell(ctr_ptr).unwrap();
                let dup = heap.consume_cell(dup_ptr).unwrap();

                let var_ptr_1 = heap.alloc_var();
                let var_ptr_2 = heap.alloc_var();
                let var_ptr_3 = heap.alloc_var();
                let var_ptr_4 = heap.alloc_var();

                let ctr_ptr_2 = heap.alloc_ctr((var_ptr_3, var_ptr_4).into());
                let dup_ptr_2 = heap.alloc_ctr((var_ptr_3, var_ptr_4).into());

                let eqn1 = Self::to_equation(ctr.0, dup_ptr.into());
                let eqn2 = Self::to_equation(ctr.1, dup_ptr_2.into());
                let eqn3 = Self::to_equation(dup.0, ctr_ptr.into());
                let eqn4 = Self::to_equation(dup.1, ctr_ptr_2.into());

                heap.set_cell(ctr_ptr, Cell::Ctr(var_ptr_1.into(), var_ptr_2.into())); // NOTE: ctr_ptr is reused here!
                heap.set_cell(dup_ptr, Cell::Dup(var_ptr_1.into(), var_ptr_2.into())); // NOTE: dup_ctr is reused here!

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
        var_ptr: VarPtr,
        cell_ptr: CellPtr,
    ) {
        self.inc_binds();

        match heap.swap_var(var_ptr, cell_ptr) {
            Some(other_cell_ptr) => {
                self.rewrite_redex(scope, heap, cell_ptr, other_cell_ptr);
                heap.free_var(var_ptr); // TODO: only free if bound variable
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
        left_ptr: VarPtr,
        right_ptr: VarPtr,
    ) {
        self.inc_connects();

        match (heap.read_var(left_ptr), heap.read_var(right_ptr)) {
            (Some(left_cell_ptr), Some(right_cell_ptr)) => {
                self.rewrite_redex(scope, heap, left_cell_ptr, right_cell_ptr);
                heap.free_var(left_ptr);
                heap.free_var(right_ptr);
            }
            (None, Some(cell_ptr)) => {
                heap.free_var(right_ptr);
                self.eval_bind(scope, heap, left_ptr, cell_ptr)
            }
            (Some(cell_ptr), None) => {
                heap.free_var(left_ptr);
                self.eval_bind(scope, heap, right_ptr, cell_ptr)
            }
            (None, None) => {
                // TODO: See #1
            }
        }
    }

    fn to_equation(left_ptr: TermPtr, right_ptr: TermPtr) -> Equation {
        match (left_ptr, right_ptr) {
            (TermPtr::CellPtr(left_ptr), TermPtr::CellPtr(right_ptr)) => Equation::Redex {
                left_ptr,
                right_ptr,
            },
            (TermPtr::CellPtr(cell_ptr), TermPtr::VarPtr(var_ptr))
            | (TermPtr::VarPtr(var_ptr), TermPtr::CellPtr(cell_ptr)) => {
                Equation::Bind { var_ptr, cell_ptr }
            }
            (TermPtr::VarPtr(left_ptr), TermPtr::VarPtr(right_ptr)) => Equation::Connect {
                left_ptr,
                right_ptr,
            },
        }
    }
}
