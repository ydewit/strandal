use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use tracing::{debug, info};

use super::{
    cell::{Cell, CellRef},
    equation::Equation,
    net::Net,
    store::Store,
    term::TermRef,
    var::VarRef,
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
                left_ref,
                right_ref,
            } => self.eval_redex(scope, store, left_ref, right_ref),
            Equation::Bind { var_ref, cell_ref } => self.eval_bind(scope, store, var_ref, cell_ref),
            Equation::Connect {
                left_ref,
                right_ref,
            } => self.eval_connect(scope, store, left_ref, right_ref),
        }
    }

    fn reduce_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_term_ref: TermRef,
        right_term_ref: TermRef,
    ) {
        match (left_term_ref, right_term_ref) {
            (TermRef::CellRef(left_ref), TermRef::CellRef(right_ref)) => {
                // spawn new rewrite thread
                self.spawn_redex(scope, store, left_ref, right_ref);
            }
            (TermRef::CellRef(cell_ref), TermRef::VarRef(var_ref))
            | (TermRef::VarRef(var_ref), TermRef::CellRef(cell_ref)) => {
                self.eval_bind(scope, store, var_ref, cell_ref);
            }
            (TermRef::VarRef(left_ref), TermRef::VarRef(right_ref)) => {
                self.eval_connect(scope, store, left_ref, right_ref);
            }
        }
    }

    fn eval_equation_for_cell<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_term_ref: TermRef,
        right_cell_ref: CellRef,
    ) {
        match left_term_ref {
            TermRef::CellRef(left_cell_ref) => {
                // spawn new rewrite thread
                self.spawn_redex(scope, store, left_cell_ref, right_cell_ref);
            }
            TermRef::VarRef(var_ref) => self.eval_bind(scope, store, var_ref, right_cell_ref),
        }
    }

    #[inline]
    fn spawn_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ref: CellRef,
        right_ref: CellRef,
    ) {
        scope.spawn(move |scope| self.eval_redex(scope, store, left_ref, right_ref));
    }

    fn eval_redex<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_cell_ref: CellRef,
        right_cell_ref: CellRef,
    ) {
        assert!(left_cell_ref != right_cell_ref);

        self.inc_redexes();

        debug!(
            "({}) eval REDEX: {}",
            self.thread_id(),
            store.display_redex(&left_cell_ref, &right_cell_ref)
        );

        // eval unboxed ERA
        match (left_cell_ref, right_cell_ref) {
            // Annihilate: ERA-ERA
            (CellRef::Era, CellRef::Era) => {
                // PUFF! Do nothing
                self.inc_erasures();

                // nothing to free since ptr are unboxed
            }
            // Commute: ERA-DUP / ERA-CTR
            (CellRef::Era, CellRef::Ref(ptr)) | (CellRef::Ref(ptr), CellRef::Era) => {
                // stats
                self.inc_comm();

                let ctr = store.consume_cell(ptr).unwrap();
                let ctr_ports = ctr.ports();
                self.reduce_equation(scope, store, CellRef::Era.into(), *ctr_ports.0);
                self.reduce_equation(scope, store, CellRef::Era.into(), *ctr_ports.1);
            }
            (CellRef::Ref(left_ptr), CellRef::Ref(right_ptr)) => {
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

                        let var_ref_1 = store.alloc_var();
                        let var_ref_2 = store.alloc_var();
                        let var_ref_3 = store.alloc_var();
                        let var_ref_4 = store.alloc_var();

                        let ctr_ref = left_cell_ref; // reuse
                        let dup_ref = right_cell_ref; // reuse
                        let ctr_ref_2 =
                            store.alloc_cell(Cell::Ctr(var_ref_3.into(), var_ref_4.into()).into());
                        let dup_ref_2 =
                            store.alloc_cell(Cell::Dup(var_ref_3.into(), var_ref_4.into()).into());

                        store.write_cell(&left_ptr, Cell::Ctr(var_ref_1.into(), var_ref_2.into())); // NOTE: ctr_ref is reused here!
                        store.write_cell(&right_ptr, Cell::Dup(var_ref_1.into(), var_ref_2.into())); // NOTE: dup_ctr is reused here!

                        self.eval_equation_for_cell(scope, store, ctr_port_0, dup_ref);
                        self.reduce_equation(scope, store, ctr_port_1, dup_ref_2.into());
                        self.eval_equation_for_cell(scope, store, dup_port_0, ctr_ref);
                        self.reduce_equation(scope, store, dup_port_1, ctr_ref_2.into());
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
        var_ref: VarRef,
        cell_ref: CellRef,
    ) {
        self.inc_binds();
        debug!(
            "({}) eval BIND: {}",
            self.thread_id(),
            store.display_bind(&var_ref, &cell_ref)
        );

        self.eval_bind_walk(scope, store, var_ref, cell_ref);
    }

    fn eval_bind_walk<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ref: VarRef,
        cell_ref: CellRef,
    ) {
        let var = store.get_var(&var_ref);
        match var.set(cell_ref.into()) {
            // walk to next var
            Some(TermRef::VarRef(other_var_ref)) => {
                store.free_var(var_ref);
                self.eval_bind_walk(scope, store, other_var_ref, cell_ref);
            }
            // spawn redex
            Some(TermRef::CellRef(other_cell_ref)) => {
                self.eval_redex(scope, store, cell_ref, other_cell_ref);
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
        var_ref: VarRef,
        other_var_ref: Option<VarRef>,
        var_value: TermRef,
    ) {
        let var = store.get_var(&var_ref);
        match var.set(var_value) {
            None => {
                // the var was unset and it is not set. This walk is done
            }
            Some(term_ref @ TermRef::CellRef(_)) => {
                store.free_var(var_ref);
                // Now we walk to the other_var_ref to set it
                match other_var_ref {
                    Some(other_var_ref) => {
                        self.eval_connect_walk(scope, store, other_var_ref, None, term_ref)
                    }
                    None => {
                        match (var_value, term_ref) {
                            (TermRef::CellRef(left_ref), TermRef::CellRef(right_ref)) => {
                                // found a redex to spawn
                                self.spawn_redex(scope, store, left_ref, right_ref);
                            }
                            (TermRef::CellRef(cell_ref), TermRef::VarRef(var_ref))
                            | (TermRef::VarRef(var_ref), TermRef::CellRef(cell_ref)) => {
                                // found a bind, evaluate it
                                self.eval_bind(scope, store, var_ref, cell_ref)
                            }
                            (
                                left_value @ TermRef::VarRef(left_ref),
                                TermRef::VarRef(right_ref),
                            ) => {
                                // found two vars again? Start walking again
                                self.eval_connect_walk(
                                    scope,
                                    store,
                                    right_ref,
                                    Some(left_ref),
                                    left_value,
                                );
                            }
                        }
                        // no other var to walk. We are done.
                    }
                };
            }
            Some(TermRef::VarRef(next_var_ref)) => {
                // the var was already connected to another var
                store.free_var(var_ref);
                // continue walking to the next var
                self.eval_connect_walk(scope, store, next_var_ref, other_var_ref, var_value)
            }
        }
    }

    fn eval_connect<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_var_ref: VarRef,
        right_var_ref: VarRef,
    ) {
        self.inc_connects();

        debug!(
            "({}) eval CONNECT: {} ↔ {}",
            self.thread_id(),
            left_var_ref,
            right_var_ref
        );

        self.eval_connect_walk(
            scope,
            store,
            right_var_ref,
            Some(left_var_ref),
            left_var_ref.into(),
        );
    }
}
