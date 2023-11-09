use std::time::Instant;

use tracing::{debug, info};

use crate::strandal::{display::CellDisplay, display::VarDisplay, stats::Stats, var::VarValue};

use super::{
    net::Net,
    stats::{GlobalStats, LocalStats},
    store::{FreePtrs, Ptr, Store},
    term::{Cell, Term, TermPtr},
    var::Var,
};

pub struct Runtime {
    pub stats: GlobalStats,
}
impl Runtime {
    pub fn new() -> Self {
        Runtime {
            stats: GlobalStats::new(),
        }
    }

    fn free_ptrs<'scope>(&'scope self, store: &'scope Store, free_ptrs: &mut FreePtrs) {
        while let Some(ptr) = free_ptrs.pop() {
            store.free(ptr);
        }
    }

    pub fn eval(&mut self, net: &mut Net) {
        let now = Instant::now();
        rayon::scope(|scope| {
            net.body.drain(..).for_each(|eqn| {
                // eval this equation
                self.spawn_eval_equation(scope, &net.store, eqn.left, eqn.right, None);
            });
        });
        info!(
            "Net evaluated in {:0.0} microseconds",
            now.elapsed().as_nanos() / 1000
        );
    }

    fn spawn_eval_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left: TermPtr,
        right: TermPtr,
        free_ptrs: Option<FreePtrs>,
    ) {
        scope.spawn(move |scope| {
            let mut free_ptrs = free_ptrs.unwrap_or_else(|| FreePtrs::new());
            let mut stats = LocalStats::new();
            // eval this equation
            self.eval_equation(scope, store, left, right, &mut free_ptrs, &mut stats);

            // free all unused free ptrs
            self.free_ptrs(store, &mut free_ptrs);
            // update global stats
            self.stats.update(stats);
        })
    }

    #[inline]
    fn spawn_eval_cell_term<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        cell_ptr: Option<Ptr>,
        cell: Cell,
        term_ptr: TermPtr,
        mut free_ptrs: FreePtrs,
    ) {
        scope.spawn(move |scope| {
            let mut stats = LocalStats::new();
            self.eval_cell_term(
                scope,
                store,
                cell_ptr,
                cell,
                term_ptr,
                &mut free_ptrs,
                &mut stats,
            );
            self.stats.update(stats);
            self.free_ptrs(store, &mut free_ptrs);
        });
    }

    #[inline]
    fn spawn_eval_era_term<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        term_ptr: TermPtr,
        mut free_ptrs: FreePtrs,
    ) {
        scope.spawn(move |scope| {
            let mut stats = LocalStats::new();
            self.eval_era_term(scope, store, term_ptr, &mut free_ptrs, &mut stats);
            self.stats.update(stats);
        });
    }

    // ------------------- CONNECT --------------------------
    fn connect_vars<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Ptr,
        left: &Var,
        right_ptr: Ptr,
        right: &Var,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_connects();

        debug!(
            "({:02}) eval CONNECT : {} ↔ {}",
            self.thread_id(),
            VarDisplay(left_ptr, left),
            VarDisplay(right_ptr, right)
        );

        match self.walk_var(
            scope,
            store,
            right_ptr,
            right,
            None,
            free_ptrs,
            stats,
            |var, _| var.link(left_ptr),
        ) {
            VarValue::Era => {
                // the right var was alredy set, this connect turns into a bind
                self.bind_era(scope, store, left_ptr, left, free_ptrs, stats)
            }
            VarValue::Cell(cell_ptr) => {
                // the right var was already set, so this connect turns into a bind
                let cell = self.get_cell(store, cell_ptr);
                self.bind_cell(
                    scope,
                    store,
                    left_ptr,
                    left,
                    Some(cell_ptr),
                    *cell,
                    free_ptrs,
                    stats,
                );
            }
            VarValue::Var(right_ptr_set) => {
                // now right var is set to link to the left one: we need set the left var to link to the right one too
                match self.walk_var(
                    scope,
                    store,
                    left_ptr,
                    left,
                    None,
                    free_ptrs,
                    stats,
                    |var, _| var.link(right_ptr_set),
                ) {
                    VarValue::Var(_) => {
                        // TODO set completed?
                    }
                    VarValue::Era => {
                        // what if walking updated a different var?
                        // TODO if diff, we are loading the var twice: could we return the var reference instead?
                        let right_set = if right_ptr_set != right_ptr {
                            self.get_var(store, right_ptr_set)
                        } else {
                            right
                        };
                        self.bind_era(scope, store, right_ptr_set, right_set, free_ptrs, stats)
                    }
                    VarValue::Cell(cell_ptr) => {
                        // what if walking updated a different var?
                        // TODO if diff, we are loading the var twice: could we return the var reference instead?
                        let right_set = if right_ptr_set != right_ptr {
                            self.get_var(store, right_ptr_set)
                        } else {
                            right
                        };
                        let cell = self.get_cell(store, cell_ptr);
                        self.bind_cell(
                            scope,
                            store,
                            right_ptr_set,
                            right_set,
                            Some(cell_ptr),
                            *cell,
                            free_ptrs,
                            stats,
                        )
                    }
                }
            }
        }
    }

    // ------------------- BIND --------------------------

    fn bind_era<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: Ptr,
        var: &Var,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_binds();

        debug!(
            "({:02}) eval BIND    : {} ← {}",
            self.thread_id(),
            VarDisplay(var_ptr, var),
            CellDisplay::ERA_SYMBOL
        );

        match self.walk_var(
            scope,
            store,
            var_ptr,
            var,
            None,
            free_ptrs,
            stats,
            |var, _| var.assign_era(),
        ) {
            VarValue::Era => self.anni_era_era(scope, store, free_ptrs, stats),
            VarValue::Cell(cell_ptr) => {
                let cell: &Cell = store.get(cell_ptr).as_ref().unwrap().try_into().unwrap();
                self.eval_era_cell(scope, store, Some(cell_ptr), *cell, free_ptrs, stats)
            }
            VarValue::Var(_) => {
                // done
            }
        }
    }

    fn bind_cell<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: Ptr,
        var: &Var,
        cell_ptr: Option<Ptr>,
        cell: Cell,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_binds();

        debug!(
            "({:02}) eval BIND    : {} ← {}",
            self.thread_id(),
            VarDisplay(var_ptr, var),
            CellDisplay(store, cell_ptr, &cell)
        );

        let right_value = self.walk_var(
            scope,
            store,
            var_ptr,
            var,
            None,
            free_ptrs,
            stats,
            |var, stats| {
                let cell_ptr =
                    cell_ptr.map_or_else(|| self.alloc_cell(store, cell.into(), stats), |ptr| ptr);
                var.assign_cell(cell_ptr)
            },
        );

        match right_value {
            VarValue::Var(_) => {
                // var set
            }
            VarValue::Era => self.eval_era_cell(scope, store, cell_ptr, cell, free_ptrs, stats),
            VarValue::Cell(other_cell_ptr) => {
                let other_cell: &Cell = store
                    .get(other_cell_ptr)
                    .as_ref()
                    .unwrap()
                    .try_into()
                    .unwrap();
                self.eval_cell_cell(
                    scope,
                    store,
                    cell_ptr,
                    cell,
                    Some(other_cell_ptr),
                    *other_cell,
                    free_ptrs,
                    stats,
                )
            }
        }
    }

    fn walk_var<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: Ptr,
        var: &Var,
        // need to avoid loops when traversing linked vars
        previous_ptr: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
        assign_var: impl Fn(&Var, &mut LocalStats) -> Option<VarValue>,
    ) -> VarValue {
        // we either have a ptr for this cell or we need to allocate it
        // TODO this alloc_cell could be wasted if the Var already has a cell! Should we read first?
        // let cell_ptr = cell_ptr.map_or_else(|| self.alloc_cell(store, cell.into()), |ptr| ptr);
        match assign_var(var, stats) {
            None => {
                return VarValue::Var(var_ptr);
            }
            Some(VarValue::Var(other_var_ptr)) => {
                // are we going in circles?
                // TODO: this will only check for direct cycles: could there be an indirect cycle?
                if Some(other_var_ptr) != previous_ptr {
                    // was linked
                    store.free(var_ptr);
                    let other_var = self.get_var(store, other_var_ptr);
                    // walk to the next var
                    return self.walk_var(
                        scope,
                        store,
                        other_var_ptr,
                        other_var,
                        Some(var_ptr),
                        free_ptrs,
                        stats,
                        assign_var,
                    );
                } else {
                    // var already set : in its final state
                    return VarValue::Var(var_ptr);
                }
            }
            Some(val @ VarValue::Era) => {
                // var already set : in its final state
                free_ptrs.push(var_ptr);
                return val;
            }
            Some(val @ VarValue::Cell(_)) => {
                // var already set : in its final state
                free_ptrs.push(var_ptr);
                return val;
            }
        }
    }

    // --------------------- EVALS ---------------------

    fn eval_equation<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left: TermPtr,
        right: TermPtr,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        return match left {
            TermPtr::Era => self.eval_era_term(scope, store, right, free_ptrs, stats),
            TermPtr::Ptr(ptr) => match store.get(ptr).as_ref().unwrap() {
                Term::Cell(cell) => {
                    // copy cell to the stack
                    self.eval_cell_term(scope, store, Some(ptr), *cell, right, free_ptrs, stats)
                }
                Term::Var(var) => {
                    self.eval_var_term(scope, store, ptr, var, right, free_ptrs, stats)
                }
            },
        };
    }

    #[inline]
    fn eval_era_term<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        term_ptr: TermPtr,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match term_ptr {
            TermPtr::Era => self.anni_era_era(scope, store, free_ptrs, stats),
            TermPtr::Ptr(ptr) => self.eval_era_ptr(scope, store, ptr, free_ptrs, stats),
        }
    }

    #[inline]
    fn eval_cell_term<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        cell_ptr: Option<Ptr>,
        cell: Cell,
        term_ptr: TermPtr,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match term_ptr {
            TermPtr::Era => self.eval_era_cell(scope, store, cell_ptr, cell, free_ptrs, stats),
            TermPtr::Ptr(ptr) => match store.get(ptr).as_ref().unwrap() {
                Term::Cell(other_cell) => {
                    // copy other_cell to the stack
                    self.eval_cell_cell(
                        scope,
                        store,
                        cell_ptr,
                        cell,
                        Some(ptr),
                        *other_cell,
                        free_ptrs,
                        stats,
                    )
                }
                Term::Var(var) => {
                    self.bind_cell(scope, store, ptr, var, cell_ptr, cell, free_ptrs, stats)
                }
            },
        }
    }

    #[inline]
    fn eval_var_term<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        var_ptr: Ptr,
        var: &Var,
        term_ptr: TermPtr,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match term_ptr {
            TermPtr::Era => self.bind_era(scope, store, var_ptr, var, free_ptrs, stats),
            TermPtr::Ptr(ptr) => match store.get(ptr).as_ref().unwrap() {
                Term::Cell(cell) => {
                    // copy cell to the stack
                    self.bind_cell(
                        scope,
                        store,
                        var_ptr,
                        var,
                        Some(ptr),
                        *cell,
                        free_ptrs,
                        stats,
                    )
                }
                Term::Var(other_var) => {
                    self.connect_vars(scope, store, var_ptr, var, ptr, other_var, free_ptrs, stats)
                }
            },
        }
    }

    #[inline]
    fn eval_era_ptr<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        ptr: Ptr,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match store.get(ptr).as_ref().unwrap() {
            Term::Var(var) => self.bind_era(scope, store, ptr, var, free_ptrs, stats),
            Term::Cell(cell) => {
                // copy Cell to the stack
                self.eval_era_cell(scope, store, Some(ptr), *cell, free_ptrs, stats)
            }
        }
    }

    #[inline]
    fn eval_era_cell<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        cell_ptr: Option<Ptr>,
        cell: Cell,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match cell {
            Cell::Dup(ports, lbl) => {
                self.commute_era_dup(scope, store, cell_ptr, ports, lbl, free_ptrs, stats);
            }
            Cell::App(ports) => self.comm_era_app(scope, store, cell_ptr, ports, free_ptrs, stats),
            Cell::Lam(ports) => self.comm_era_lam(scope, store, cell_ptr, ports, free_ptrs, stats),
        }
    }

    #[inline]
    fn eval_cell_cell<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left: Cell,
        right_ptr: Option<Ptr>,
        right: Cell,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        match (left, right) {
            // ANNIHILATE APP-APP
            (Cell::App(left_ports), Cell::App(right_ports)) => {
                self.anni_app_app(
                    scope,
                    store,
                    left_ptr,
                    left_ports,
                    right_ptr,
                    right_ports,
                    free_ptrs,
                    stats,
                );
            }
            // ANNIHILATE LAM-LAM
            (Cell::Lam(left_ports), Cell::Lam(right_ports)) => self.anni_lam_lam(
                scope,
                store,
                left_ptr,
                left_ports,
                right_ptr,
                right_ports,
                free_ptrs,
                stats,
            ),
            // ANNIHILATE or COMMUTE DUP-DUP
            (Cell::Dup(left_ports, left_lbl), Cell::Dup(right_ports, right_lbl)) => self
                .reduce_dup_dup(
                    scope,
                    store,
                    left_ptr,
                    left_ports,
                    left_lbl,
                    right_ptr,
                    right_ports,
                    right_lbl,
                    free_ptrs,
                    stats,
                ),
            // COMMUTE APP-DUP
            (Cell::App(app_ports), Cell::Dup(dup_ports, dup_lbl))
            | (Cell::Dup(dup_ports, dup_lbl), Cell::App(app_ports)) => {
                self.commute_app_dup(
                    scope, store, right_ptr, app_ports, left_ptr, dup_ports, dup_lbl, free_ptrs,
                    stats,
                );
            }
            (Cell::App(app_ports), Cell::Lam(lam_ports))
            | (Cell::Lam(lam_ports), Cell::App(app_ports)) => self.commute_app_lam(
                scope, store, right_ptr, app_ports, left_ptr, lam_ports, free_ptrs, stats,
            ),
            (Cell::Dup(dup_ports, dup_lbl), Cell::Lam(lam_ports))
            | (Cell::Lam(lam_ports), Cell::Dup(dup_ports, dup_lbl)) => self.commute_lam_dup(
                scope, store, left_ptr, lam_ports, right_ptr, dup_ports, dup_lbl, free_ptrs, stats,
            ),
        }
    }

    // ------------------- REDUCTIONS ----------------------------------

    #[inline]
    fn anni_era_era<'scope>(
        &'scope self,
        _scope: &rayon::Scope<'scope>,
        _store: &'scope Store,
        _free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_anni_era_era();

        debug!(
            "({:02}) anni ERA-ERA : {} <- {}",
            self.thread_id(),
            CellDisplay::ERA_SYMBOL,
            CellDisplay::ERA_SYMBOL
        );
    }

    #[inline]
    fn anni_lam_lam<'scope>(
        &'scope self,
        _scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left_ports: Option<(TermPtr, TermPtr)>,
        right_ptr: Option<Ptr>,
        right_ports: Option<(TermPtr, TermPtr)>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_anni_lam_lam();

        debug!(
            "({:02}) anni LAM-LAM : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, left_ptr, &Cell::Lam(left_ports)),
            CellDisplay(store, right_ptr, &Cell::Lam(right_ports)),
        );

        left_ptr.map(|ptr| free_ptrs.push(ptr));
        right_ptr.map(|ptr| free_ptrs.push(ptr));
    }

    #[inline]
    fn anni_app_app<'scope>(
        &'scope self,
        _scope: &rayon::Scope<'scope>,
        _store: &'scope Store,
        left_ptr: Option<Ptr>,
        _left_ports: Option<(TermPtr, TermPtr)>,
        right_ptr: Option<Ptr>,
        _right_ports: Option<(TermPtr, TermPtr)>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_anni_app_app();

        debug!(
            "({:02}) anni APP-APP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay::LAM_SYMBOL,
            CellDisplay::LAM_SYMBOL
        );

        left_ptr.map(|ptr| free_ptrs.push(ptr));
        right_ptr.map(|ptr| free_ptrs.push(ptr));
    }

    /// Reduce a DUP-DUP pair, which may be annihilated or commuted depending
    /// on the label.
    ///
    /// If the labels are equal, the DUP-DUP pair is annihilated, otherwise commuted.
    ///
    #[inline]
    fn reduce_dup_dup<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left_ports: Option<(TermPtr, TermPtr)>,
        left_lbl: Option<Ptr>,
        right_ptr: Option<Ptr>,
        right_ports: Option<(TermPtr, TermPtr)>,
        right_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        if left_lbl == right_lbl {
            stats.inc_anni_dup_dup();
            self.anni_dup_dup(
                scope,
                store,
                left_ptr,
                left_ports,
                left_lbl,
                right_ptr,
                right_ports,
                right_lbl,
                free_ptrs,
                stats,
            )
        } else {
            stats.inc_comm_dup_dup();
            self.comm_dup_dup(
                scope,
                store,
                left_ptr,
                left_ports,
                left_lbl,
                right_ptr,
                right_ports,
                right_lbl,
                free_ptrs,
                stats,
            )
        }
    }

    fn anni_dup_dup<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left_ports: Option<(TermPtr, TermPtr)>,
        left_lbl: Option<Ptr>,
        right_ptr: Option<Ptr>,
        right_ports: Option<(TermPtr, TermPtr)>,
        right_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        left_ptr.map(|ptr| free_ptrs.push(ptr));
        right_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) anni DUP-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, left_ptr, &Cell::Dup(None, left_lbl)),
            CellDisplay(store, right_ptr, &Cell::Dup(None, right_lbl))
        );

        match (left_ports, right_ports) {
            // Disconnected NET: (Dup a a ?) ⋈ (Dup b b ?)
            (None, None) => {}
            // (Dup a a ?) ⋈ (Dup b c ?)
            (Some((p0, p1)), None) | (None, Some((p0, p1))) => {
                self.eval_equation(scope, store, p0, p1, free_ptrs, stats);
            }
            (Some((left_p0, left_p1)), Some((right_p0, right_p1))) => {
                // ANNIHILATE
                self.spawn_eval_equation(
                    scope,
                    store,
                    left_p0,
                    right_p0,
                    free_ptrs.split(2).into(),
                );
                self.eval_equation(scope, store, left_p1, right_p1, free_ptrs, stats);
            }
        }
    }

    fn comm_dup_dup<'scope>(
        &'scope self,
        _scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left_ports: Option<(TermPtr, TermPtr)>,
        left_lbl: Option<Ptr>,
        right_ptr: Option<Ptr>,
        right_ports: Option<(TermPtr, TermPtr)>,
        right_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        _stats: &mut LocalStats,
    ) {
        left_ptr.map(|ptr| free_ptrs.push(ptr));
        right_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) comm DUP-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, left_ptr, &Cell::Dup(None, left_lbl)),
            CellDisplay(store, right_ptr, &Cell::Dup(None, right_lbl))
        );

        match (left_ports, right_ports) {
            // Disconnected NET: (Dup a a ?) ⋈ (Dup b b ?)
            (None, None) => {}
            // (Dup a a ?) ⋈ (Dup b c ?)
            (Some((_p0, _p1)), None) | (None, Some((_p0, _p1))) => {
                // COMMUTE
                todo!("comm dup-dup not yet implemented")
            }
            (Some((_left_p0, _left_p1)), Some((_right_p0, _right_p1))) => {
                // COMMUTE
                todo!("comm dup-dup not yet implemented")
            }
        }
    }

    #[inline]
    fn comm_era_app<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        app_ptr: Option<Ptr>,
        app_ports: Option<(TermPtr, TermPtr)>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_comm_era_app();

        app_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) comm ERA-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay::ERA_SYMBOL,
            CellDisplay(store, app_ptr, &Cell::App(app_ports))
        );

        // TODO inc ERA-APP
        match app_ports {
            Some((p0, p1)) => {
                self.spawn_eval_era_term(scope, store, p0, free_ptrs.split(2).into());
                self.eval_era_term(scope, store, p1, free_ptrs, stats);
            }
            None => {
                self.anni_era_era(scope, store, free_ptrs, stats);
            }
        }
    }

    #[inline]
    fn comm_era_lam<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        lam_ptr: Option<Ptr>,
        lam_ports: Option<(TermPtr, TermPtr)>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_comm_era_lam();

        lam_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) comm ERA-LAM : {} ⋈ {}",
            self.thread_id(),
            CellDisplay::ERA_SYMBOL,
            CellDisplay(store, lam_ptr, &Cell::Lam(lam_ports))
        );

        // TODO inc ERA-LAM
        match lam_ports {
            Some((p0, p1)) => {
                self.spawn_eval_era_term(scope, store, p0, free_ptrs.split(2).into());
                self.eval_era_term(scope, store, p1, free_ptrs, stats);
            }
            None => {
                self.anni_era_era(scope, store, free_ptrs, stats);
            }
        }
    }

    #[inline]
    fn commute_era_dup<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        dup_ptr: Option<Ptr>,
        dup_ports: Option<(TermPtr, TermPtr)>,
        dup_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_commute_era_dup();
        dup_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) comm ERA-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay::ERA_SYMBOL,
            CellDisplay(store, dup_ptr, &Cell::Dup(dup_ports, dup_lbl))
        );

        match dup_ports {
            Some((p0, p1)) => {
                self.spawn_eval_era_term(scope, store, p0, free_ptrs.split(2).into());
                self.eval_era_term(scope, store, p1, free_ptrs, stats);
            }
            None => {
                self.anni_era_era(scope, store, free_ptrs, stats);
            }
        }
    }

    #[inline]
    fn commute_app_lam<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        app_ptr: Option<Ptr>,
        app_ports: Option<(TermPtr, TermPtr)>,
        lam_ptr: Option<Ptr>,
        lam_ports: Option<(TermPtr, TermPtr)>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_comm_app_lam();
        lam_ptr.map(|ptr| free_ptrs.push(ptr));
        app_ptr.map(|ptr| free_ptrs.push(ptr));

        debug!(
            "({:02}) comm APP-LAM : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, app_ptr, &Cell::App(app_ports)),
            CellDisplay(store, lam_ptr, &Cell::Lam(lam_ports))
        );

        match (app_ports, lam_ports) {
            (Some((p0, p1)), Some((q0, q1))) => {
                self.spawn_eval_equation(scope, store, p0, q0, free_ptrs.split(2).into());
                self.eval_equation(scope, store, p1, q1, free_ptrs, stats);
            }
            (Some((p0, p1)), None) => {
                self.spawn_eval_equation(scope, store, p0, TermPtr::Era, free_ptrs.split(2).into());
                self.eval_equation(scope, store, p1, TermPtr::Era, free_ptrs, stats);
            }
            (None, Some((q0, q1))) => {
                self.spawn_eval_equation(scope, store, TermPtr::Era, q0, free_ptrs.split(2).into());
                self.eval_equation(scope, store, TermPtr::Era, q1, free_ptrs, stats);
            }
            (None, None) => {
                self.eval_equation(scope, store, TermPtr::Era, TermPtr::Era, free_ptrs, stats);
            }
        }
    }

    #[inline]
    fn commute_app_dup<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        app_ptr: Option<Ptr>,
        app_ports: Option<(TermPtr, TermPtr)>,
        dup_ptr: Option<Ptr>,
        dup_ports: Option<(TermPtr, TermPtr)>,
        dup_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_comm_app_dup();

        debug!(
            "({:02}) comm APP-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, app_ptr, &Cell::App(app_ports)),
            CellDisplay(store, dup_ptr, &Cell::Dup(dup_ports, dup_lbl))
        );

        self.commute(
            scope,
            store,
            app_ptr,
            app_ports,
            |app_ports, _| Cell::App(app_ports),
            false,
            dup_ptr,
            dup_ports,
            |dup_ports, lbl| Cell::Dup(dup_ports, lbl),
            true,
            free_ptrs,
            stats,
        )
    }

    #[inline]
    fn commute_lam_dup<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        lam_ptr: Option<Ptr>,
        lam_ports: Option<(TermPtr, TermPtr)>,
        dup_ptr: Option<Ptr>,
        dup_ports: Option<(TermPtr, TermPtr)>,
        dup_lbl: Option<Ptr>,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        stats.inc_comm_lam_dup();

        debug!(
            "({:02}) comm LAM-DUP : {} ⋈ {}",
            self.thread_id(),
            CellDisplay(store, lam_ptr, &Cell::Lam(lam_ports)),
            CellDisplay(store, dup_ptr, &Cell::Dup(dup_ports, dup_lbl))
        );

        self.commute(
            scope,
            store,
            lam_ptr,
            lam_ports,
            |ports, _| Cell::App(ports),
            false,
            dup_ptr,
            dup_ports,
            |ports, lbl| Cell::Dup(ports, lbl),
            true,
            free_ptrs,
            stats,
        )
    }

    #[inline]
    fn alloc_var(&self, store: &Store, stats: &mut LocalStats) -> Ptr {
        stats.inc_alloc_vars();
        return store.alloc(Some(Term::Var(Var::new())));
    }

    #[inline]
    fn alloc_cell(&self, store: &Store, cell: Option<Cell>, stats: &mut LocalStats) -> Ptr {
        stats.inc_alloc_cells();
        return store.alloc(cell.map(|c| Term::Cell(c)));
    }

    #[inline]
    fn get_cell<'scope>(&'scope self, store: &'scope Store, cell_ptr: Ptr) -> &Cell {
        match store.get(cell_ptr).as_ref().unwrap() {
            Term::Var(_) => panic!("Expected Cell, found Var"),
            Term::Cell(cell) => cell,
        }
    }

    #[inline]
    fn get_var<'scope>(&'scope self, store: &'scope Store, var_ptr: Ptr) -> &Var {
        match store.get(var_ptr).as_ref().unwrap() {
            Term::Var(var) => var,
            Term::Cell(_) => panic!("Expected Var, found Cell"),
        }
    }

    // #[inline]
    // fn reuse_var(&self, store: &Store, ptr: Ptr) {
    //     store.set(ptr, Term::Var(Var::new()));
    // }

    #[inline]
    fn reuse_cell(&self, store: &Store, ptr: Ptr, cell: Cell) {
        store.set(ptr, Term::Cell(cell));
    }

    #[inline]
    fn commute<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        store: &'scope Store,
        left_ptr: Option<Ptr>,
        left_ports: Option<(TermPtr, TermPtr)>,
        left_fn: impl Fn(Option<(TermPtr, TermPtr)>, Option<Ptr>) -> Cell,
        left_alloc: bool,
        right_ptr: Option<Ptr>,
        right_ports: Option<(TermPtr, TermPtr)>,
        right_fn: impl Fn(Option<(TermPtr, TermPtr)>, Option<Ptr>) -> Cell,
        right_alloc: bool,
        free_ptrs: &mut FreePtrs,
        stats: &mut LocalStats,
    ) {
        left_ptr.map(|ptr| free_ptrs.push(ptr));
        right_ptr.map(|ptr| free_ptrs.push(ptr));

        if left_ports.is_none() && right_ports.is_none() {
            // disconnected net
            // TODO: stats?
        } else {
            let x1 = TermPtr::Ptr(self.alloc_var(store, stats));
            let x2 = TermPtr::Ptr(self.alloc_var(store, stats));
            let x3 = TermPtr::Ptr(self.alloc_var(store, stats));
            let x4 = TermPtr::Ptr(self.alloc_var(store, stats));

            // duplicate left cell (we dont allocate it in the store)
            let left_1_ptr = if left_alloc {
                Some(free_ptrs.pop().unwrap()) // we know we have at least two (see above)
            } else {
                None
            };
            let left_0 = left_fn(Some((x1, x3)), left_1_ptr);
            let left_0_ptr = if left_alloc {
                Some(self.alloc_cell(store, Some(left_0), stats))
            } else {
                None
            };
            let left_1 = left_fn(Some((x4, x2)), left_0_ptr);

            // duplicate right cell (we dont allocate it in the store)
            let right_1_ptr = if right_alloc {
                Some(free_ptrs.pop().unwrap()) // we know we have at least two (see above)
            } else {
                None
            };
            let right_0 = right_fn(Some((x4, x1)), left_1_ptr);
            let right_0_ptr = if right_alloc {
                Some(self.alloc_cell(store, Some(right_0), stats))
            } else {
                None
            };
            let right_1 = left_fn(Some((x2, x3)), left_0_ptr);

            match (left_ports, right_ports) {
                (None, None) => unreachable!(),
                // left cell ports are self connected
                (None, Some((right_p0, right_p1))) => {
                    //
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        left_0,
                        right_p0,
                        free_ptrs.split(3).into(),
                    );
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        left_1,
                        right_p1,
                        free_ptrs.split(2).into(),
                    );

                    self.eval_cell_cell(
                        scope, store, None, // lives only in the stack and has no Store Ptr
                        right_0, None, // lives only in the stack and has no Store Ptr
                        right_1, free_ptrs, stats,
                    );
                }
                (Some((left_p0, left_p1)), None) => {
                    //
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        right_0,
                        left_p0,
                        free_ptrs.split(3).into(),
                    );
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        right_1,
                        left_p1,
                        free_ptrs.split(2).into(),
                    );

                    self.eval_cell_cell(
                        scope, store, None, // lives only in the stack and has no Store Ptr
                        left_0, None, // lives only in the stack and has no Store Ptr
                        left_1, free_ptrs, stats,
                    );
                }
                (Some((left_p0, left_p1)), Some((right_p0, right_p1))) => {
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        right_0,
                        left_p0,
                        free_ptrs.split(4).into(),
                    );
                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        right_1,
                        left_p1,
                        free_ptrs.split(3).into(),
                    );

                    self.spawn_eval_cell_term(
                        scope,
                        store,
                        None,
                        left_0,
                        right_p0,
                        free_ptrs.split(2).into(),
                    );
                    self.eval_cell_term(scope, store, None, left_1, right_p1, free_ptrs, stats);
                }
            }
        }
    }

    #[inline]
    fn thread_id(&self) -> usize {
        return rayon::current_thread_index().unwrap();
    }
}
