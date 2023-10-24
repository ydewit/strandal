use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use tracing::{debug, info};

use super::{
    cell::{Cell, CellPtr},
    equation::Equation,
    net::Net,
    store::Store,
    term::TermPtr,
    var::VarPtr,
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
                .for_each(|eqn| self.eval_equation(scope, &net.store, eqn));
        });
        info!(
            "Net evaluated in {:0.0} microseconds",
            now.elapsed().as_nanos() / 1000
        );
    }

    fn eval_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        eqn: Equation,
    ) {
        match eqn {
            Equation::Redex {
                left_ptr,
                right_ptr,
            } => self.eval_redex(scope, store, left_ptr, right_ptr),
            Equation::Bind { var_ptr, cell_ptr } => self.eval_bind(scope, store, var_ptr, cell_ptr),
            Equation::Connect {
                left_ptr,
                right_ptr,
            } => self.eval_connect(scope, store, left_ptr, right_ptr),
        }
    }

    fn reduce_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_term_ptr: TermPtr,
        right_term_ptr: TermPtr,
    ) {
        match (left_term_ptr, right_term_ptr) {
            (TermPtr::CellPtr(left_ptr), TermPtr::CellPtr(right_ptr)) => {
                // spawn new rewrite thread
                self.spawn_redex(scope, store, left_ptr, right_ptr);
            }
            (TermPtr::CellPtr(cell_ptr), TermPtr::VarPtr(var_ptr))
            | (TermPtr::VarPtr(var_ptr), TermPtr::CellPtr(cell_ptr)) => {
                self.eval_bind(scope, store, var_ptr, cell_ptr);
            }
            (TermPtr::VarPtr(left_ptr), TermPtr::VarPtr(right_ptr)) => {
                self.eval_connect(scope, store, left_ptr, right_ptr);
            }
        }
    }

    fn eval_equation_for_cell<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_term_ptr: TermPtr,
        right_cell_ptr: CellPtr,
    ) {
        match left_term_ptr {
            TermPtr::CellPtr(left_cell_ptr) => {
                // spawn new rewrite thread
                self.spawn_redex(scope, store, left_cell_ptr, right_cell_ptr);
            }
            TermPtr::VarPtr(var_ptr) => self.eval_bind(scope, store, var_ptr, right_cell_ptr),
        }
    }

    #[inline]
    fn spawn_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: CellPtr,
        right_ptr: CellPtr,
    ) {
        scope.spawn(move |scope| self.eval_redex(scope, store, left_ptr, right_ptr));
    }

    fn eval_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_cell_ptr: CellPtr,
        right_cell_ptr: CellPtr,
    ) {
        assert!(left_cell_ptr != right_cell_ptr);

        self.inc_redexes();

        debug!(
            "({}) eval REDEX: {}",
            self.thread_id(),
            store.display_redex(&left_cell_ptr, &right_cell_ptr)
        );

        // eval unboxed ERA
        match (left_cell_ptr, right_cell_ptr) {
            // Annihilate: ERA-ERA
            (CellPtr::Era, CellPtr::Era) => {
                // PUFF! Do nothing
                self.inc_erasures();

                // nothing to free since ptr are unboxed
            }
            // Commute: ERA-DUP / ERA-CTR
            (CellPtr::Era, CellPtr::Ref(ptr)) | (CellPtr::Ref(ptr), CellPtr::Era) => {
                // stats
                self.inc_comm();

                let ctr = store.consume_cell(ptr).unwrap();
                let ctr_ports = ctr.ports();
                self.reduce_equation(scope, store, CellPtr::Era.into(), *ctr_ports.0);
                self.reduce_equation(scope, store, CellPtr::Era.into(), *ctr_ports.1);
            }
            (CellPtr::Ref(left_ptr), CellPtr::Ref(right_ptr)) => {
                let left_cell = store.consume_cell(left_ptr).unwrap();
                let right_cell = store.consume_cell(right_ptr).unwrap();
                match (left_cell, right_cell) {
                    // Annihilate: CTR-CTR / DUP-DUP
                    (
                        Cell::Ctr(left_port_0, left_port_1),
                        Cell::Ctr(right_port_0, right_port_1),
                    )
                    | (
                        Cell::Dup(left_port_0, left_port_1),
                        Cell::Dup(right_port_0, right_port_1),
                    ) => {
                        self.inc_annihilations();

                        self.reduce_equation(scope, store, left_port_0, right_port_0);
                        self.reduce_equation(scope, store, left_port_1, right_port_1);
                    }

                    // Commute: CTR-DUP
                    (Cell::Ctr(ctr_port_0, ctr_port_1), Cell::Dup(dup_port_0, dup_port_1))
                    | (Cell::Dup(dup_port_0, dup_port_1), Cell::Ctr(ctr_port_0, ctr_port_1)) => {
                        self.inc_comm();

                        let var_ptr_1 = store.alloc_var();
                        let var_ptr_2 = store.alloc_var();
                        let var_ptr_3 = store.alloc_var();
                        let var_ptr_4 = store.alloc_var();

                        let ctr_ptr = left_cell_ptr; // reuse
                        let dup_ptr = right_cell_ptr; // reuse
                        let ctr_ptr_2 =
                            store.alloc_cell(Cell::Ctr(var_ptr_3.into(), var_ptr_4.into()).into());
                        let dup_ptr_2 =
                            store.alloc_cell(Cell::Dup(var_ptr_3.into(), var_ptr_4.into()).into());

                        store.write_cell(&left_ptr, Cell::Ctr(var_ptr_1.into(), var_ptr_2.into())); // NOTE: ctr_ptr is reused here!
                        store.write_cell(&right_ptr, Cell::Dup(var_ptr_1.into(), var_ptr_2.into())); // NOTE: dup_ctr is reused here!

                        self.eval_equation_for_cell(scope, store, ctr_port_0, dup_ptr);
                        self.reduce_equation(scope, store, ctr_port_1, dup_ptr_2.into());
                        self.eval_equation_for_cell(scope, store, dup_port_0, ctr_ptr);
                        self.reduce_equation(scope, store, dup_port_1, ctr_ptr_2.into());
                    }
                }
                // TODO reuse?
                store.free_cell(left_ptr); // TODO: Can we reuse this?
                store.free_cell(right_ptr); // TODO can we reuse this?
            }
        }
    }

    fn eval_bind<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: VarPtr,
        cell_ptr: CellPtr,
    ) {
        self.inc_binds();
        debug!(
            "({}) eval BIND: {}",
            self.thread_id(),
            store.display_bind(&var_ptr, &cell_ptr)
        );

        self.eval_bind_walk(scope, store, var_ptr, cell_ptr);
    }

    fn eval_bind_walk<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: VarPtr,
        cell_ptr: CellPtr,
    ) {
        let var = store.get_var(&var_ptr);
        match var.set(cell_ptr.into()) {
            // walk to next var
            Some(TermPtr::VarPtr(other_var_ptr)) => {
                store.free_var(var_ptr);
                self.eval_bind_walk(scope, store, other_var_ptr, cell_ptr);
            }
            // spawn redex
            Some(TermPtr::CellPtr(other_cell_ptr)) => {
                self.eval_redex(scope, store, cell_ptr, other_cell_ptr);
            }
            // set done
            None => {
                // value set
            }
        }
    }

    fn eval_connect_walk<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: VarPtr,
        other_var_ptr: Option<VarPtr>,
        var_value: TermPtr,
    ) {
        let var = store.get_var(&var_ptr);
        match var.set(var_value) {
            None => {
                // the var was unset and it is not set. This walk is done
            }
            Some(term_ptr @ TermPtr::CellPtr(_)) => {
                store.free_var(var_ptr);
                // Now we walk to the other_var_ptr to set it
                match other_var_ptr {
                    Some(other_var_ptr) => {
                        self.eval_connect_walk(scope, store, other_var_ptr, None, term_ptr)
                    }
                    None => {
                        match (var_value, term_ptr) {
                            (TermPtr::CellPtr(left_ptr), TermPtr::CellPtr(right_ptr)) => {
                                // found a redex to spawn
                                self.spawn_redex(scope, store, left_ptr, right_ptr);
                            }
                            (TermPtr::CellPtr(cell_ptr), TermPtr::VarPtr(var_ptr))
                            | (TermPtr::VarPtr(var_ptr), TermPtr::CellPtr(cell_ptr)) => {
                                // found a bind, evaluate it
                                self.eval_bind(scope, store, var_ptr, cell_ptr)
                            }
                            (
                                left_value @ TermPtr::VarPtr(left_ptr),
                                TermPtr::VarPtr(right_ptr),
                            ) => {
                                // found two vars again? Start walking again
                                self.eval_connect_walk(
                                    scope,
                                    store,
                                    right_ptr,
                                    Some(left_ptr),
                                    left_value,
                                );
                            }
                        }
                        // no other var to walk. We are done.
                    }
                };
            }
            Some(TermPtr::VarPtr(next_var_ptr)) => {
                // the var was already connected to another var
                store.free_var(var_ptr);
                // continue walking to the next var
                self.eval_connect_walk(scope, store, next_var_ptr, other_var_ptr, var_value)
            }
        }
    }

    fn eval_connect<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_var_ptr: VarPtr,
        right_var_ptr: VarPtr,
    ) {
        self.inc_connects();

        debug!(
            "({}) eval CONNECT: {} ↔ {}",
            self.thread_id(),
            left_var_ptr,
            right_var_ptr
        );

        self.eval_connect_walk(
            scope,
            store,
            right_var_ptr,
            Some(left_var_ptr),
            left_var_ptr.into(),
        );
    }
}
