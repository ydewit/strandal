use std::{
    fmt::{Display, Formatter},
    sync::atomic::{AtomicUsize, Ordering},
};

pub trait Stats {
    fn inc_anni_era_era(&mut self);

    fn inc_anni_app_app(&mut self);

    fn inc_anni_lam_lam(&mut self);

    fn inc_anni_dup_dup(&mut self);

    fn inc_comm_dup_dup(&mut self);

    fn inc_comm_era_app(&mut self);

    fn inc_comm_era_lam(&mut self);

    fn inc_commute_era_dup(&mut self);

    fn inc_comm_app_lam(&mut self);

    fn inc_comm_app_dup(&mut self);

    fn inc_comm_lam_dup(&mut self);

    fn inc_binds(&mut self);

    fn inc_connects(&mut self);

    fn inc_alloc_cells(&mut self);

    fn inc_alloc_vars(&mut self);
}

pub struct GlobalStats {
    anni_era_era: AtomicUsize,
    anni_app_app: AtomicUsize,
    anni_lam_lam: AtomicUsize,
    anni_dup_dup: AtomicUsize,
    comm_dup_dup: AtomicUsize,
    comm_era_app: AtomicUsize,
    comm_era_lam: AtomicUsize,
    comm_era_dup: AtomicUsize,
    comm_app_lam: AtomicUsize,
    comm_app_dup: AtomicUsize,
    comm_lam_dup: AtomicUsize,
    binds: AtomicUsize,
    connects: AtomicUsize,
    alloc_vars: AtomicUsize,
    alloc_cells: AtomicUsize,
}

impl GlobalStats {
    pub fn new() -> Self {
        Self {
            anni_era_era: AtomicUsize::new(0),
            anni_app_app: AtomicUsize::new(0),
            anni_lam_lam: AtomicUsize::new(0),
            anni_dup_dup: AtomicUsize::new(0),
            comm_dup_dup: AtomicUsize::new(0),
            comm_era_app: AtomicUsize::new(0),
            comm_era_lam: AtomicUsize::new(0),
            comm_era_dup: AtomicUsize::new(0),
            comm_app_lam: AtomicUsize::new(0),
            comm_app_dup: AtomicUsize::new(0),
            comm_lam_dup: AtomicUsize::new(0),
            binds: AtomicUsize::new(0),
            connects: AtomicUsize::new(0),
            alloc_vars: AtomicUsize::new(0),
            alloc_cells: AtomicUsize::new(0),
        }
    }
}

impl GlobalStats {
    pub fn annihilations(&self) -> usize {
        self.anni_era_era() + self.anni_app_app() + self.anni_lam_lam() + self.anni_dup_dup()
    }

    pub fn commutations(&self) -> usize {
        self.comm_dup_dup()
            + self.comm_era_app()
            + self.comm_era_lam()
            + self.comm_era_dup()
            + self.comm_app_lam()
            + self.comm_app_dup()
            + self.comm_lam_dup()
    }

    pub fn allocs(&self) -> usize {
        self.alloc_vars() + self.alloc_cells()
    }

    pub fn update(&self, stats: LocalStats) {
        self.anni_era_era
            .fetch_add(stats.anni_era_era, Ordering::Relaxed);
        self.anni_app_app
            .fetch_add(stats.anni_app_app, Ordering::Relaxed);
        self.anni_lam_lam
            .fetch_add(stats.anni_lam_lam, Ordering::Relaxed);
        self.anni_dup_dup
            .fetch_add(stats.anni_dup_dup, Ordering::Relaxed);
        self.comm_dup_dup
            .fetch_add(stats.comm_dup_dup, Ordering::Relaxed);
        self.comm_era_app
            .fetch_add(stats.comm_era_app, Ordering::Relaxed);
        self.comm_era_lam
            .fetch_add(stats.comm_era_lam, Ordering::Relaxed);
        self.comm_era_dup
            .fetch_add(stats.comm_era_dup, Ordering::Relaxed);
        self.comm_app_lam
            .fetch_add(stats.comm_app_lam, Ordering::Relaxed);
        self.comm_app_dup
            .fetch_add(stats.comm_app_dup, Ordering::Relaxed);
        self.comm_lam_dup
            .fetch_add(stats.comm_lam_dup, Ordering::Relaxed);
        self.binds.fetch_add(stats.binds, Ordering::Relaxed);
        self.connects.fetch_add(stats.connects, Ordering::Relaxed);
        self.alloc_cells
            .fetch_add(stats.alloc_cells, Ordering::Relaxed);
        self.alloc_vars
            .fetch_add(stats.alloc_vars, Ordering::Relaxed);
    }

    pub fn anni_era_era(&self) -> usize {
        self.anni_era_era.load(Ordering::Relaxed)
    }

    pub fn anni_app_app(&self) -> usize {
        self.anni_app_app.load(Ordering::Relaxed)
    }

    pub fn anni_lam_lam(&self) -> usize {
        self.anni_lam_lam.load(Ordering::Relaxed)
    }

    pub fn anni_dup_dup(&self) -> usize {
        self.anni_dup_dup.load(Ordering::Relaxed)
    }

    pub fn comm_dup_dup(&self) -> usize {
        self.comm_dup_dup.load(Ordering::Relaxed)
    }

    pub fn comm_era_app(&self) -> usize {
        self.comm_era_app.load(Ordering::Relaxed)
    }

    pub fn comm_era_lam(&self) -> usize {
        self.comm_era_lam.load(Ordering::Relaxed)
    }

    pub fn comm_era_dup(&self) -> usize {
        self.comm_era_dup.load(Ordering::Relaxed)
    }

    pub fn comm_app_lam(&self) -> usize {
        self.comm_app_lam.load(Ordering::Relaxed)
    }

    pub fn comm_app_dup(&self) -> usize {
        self.comm_app_dup.load(Ordering::Relaxed)
    }

    pub fn comm_lam_dup(&self) -> usize {
        self.comm_lam_dup.load(Ordering::Relaxed)
    }

    pub fn binds(&self) -> usize {
        self.binds.load(Ordering::Relaxed)
    }

    pub fn connects(&self) -> usize {
        self.connects.load(Ordering::Relaxed)
    }

    pub fn alloc_cells(&self) -> usize {
        self.alloc_cells.load(Ordering::Relaxed)
    }

    pub fn alloc_vars(&self) -> usize {
        self.alloc_vars.load(Ordering::Relaxed)
    }
}

pub struct LocalStats {
    anni_era_era: usize,
    anni_app_app: usize,
    anni_lam_lam: usize,
    anni_dup_dup: usize,
    comm_dup_dup: usize,
    comm_era_app: usize,
    comm_era_lam: usize,
    comm_era_dup: usize,
    comm_app_lam: usize,
    comm_app_dup: usize,
    comm_lam_dup: usize,
    binds: usize,
    connects: usize,
    alloc_cells: usize,
    alloc_vars: usize,
}
impl LocalStats {
    pub fn new() -> Self {
        Self {
            anni_era_era: 0,
            anni_app_app: 0,
            anni_lam_lam: 0,
            anni_dup_dup: 0,
            comm_dup_dup: 0,
            comm_era_app: 0,
            comm_era_lam: 0,
            comm_era_dup: 0,
            comm_app_lam: 0,
            comm_app_dup: 0,
            comm_lam_dup: 0,
            binds: 0,
            connects: 0,
            alloc_cells: 0,
            alloc_vars: 0,
        }
    }
}

impl Stats for LocalStats {
    fn inc_anni_era_era(&mut self) {
        self.anni_era_era += 1;
    }

    fn inc_anni_app_app(&mut self) {
        self.anni_app_app += 1;
    }

    fn inc_anni_lam_lam(&mut self) {
        self.anni_lam_lam += 1;
    }

    fn inc_anni_dup_dup(&mut self) {
        self.anni_dup_dup += 1;
    }

    fn inc_comm_dup_dup(&mut self) {
        self.comm_dup_dup += 1;
    }

    fn inc_comm_era_app(&mut self) {
        self.comm_era_app += 1;
    }

    fn inc_comm_era_lam(&mut self) {
        self.comm_era_lam += 1;
    }

    fn inc_commute_era_dup(&mut self) {
        self.comm_era_dup += 1;
    }

    fn inc_comm_app_lam(&mut self) {
        self.comm_app_lam += 1;
    }

    fn inc_comm_app_dup(&mut self) {
        self.comm_app_dup += 1;
    }

    fn inc_comm_lam_dup(&mut self) {
        self.comm_lam_dup += 1;
    }

    fn inc_binds(&mut self) {
        self.binds += 1;
    }

    fn inc_connects(&mut self) {
        self.connects += 1;
    }

    fn inc_alloc_cells(&mut self) {
        self.alloc_cells += 1;
    }

    fn inc_alloc_vars(&mut self) {
        self.alloc_vars += 1;
    }
}

impl Display for GlobalStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SUMMARY | annis: {}, comms: {}, binds: {}, connects: {}, allocs: {}\nANNIS   | ERA-ERA: {}, LAM-LAM: {}, APP-APP: {}, DUP-DUP: {}\nCOMMS   | ERA-APP: {}, ERA-LAM: {}, ERA-DUP: {}, APP-LAM: {}, APP-DUP: {}, LAM-DUP: {}",
            self.annihilations(),
            self.commutations(),
            self.binds(),
            self.connects(),
            self.allocs(),
            self.anni_era_era(),
            self.anni_lam_lam(),
            self.anni_app_app(),
            self.anni_dup_dup(),
            self.comm_era_app(),
            self.comm_era_lam(),
            self.comm_era_dup(),
            self.comm_app_lam(),
            self.comm_app_dup(),
            self.comm_lam_dup(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::strandal::stats::{GlobalStats, LocalStats, Stats};

    // use super::*;

    #[test]
    fn test_stats() {
        let global_stats = GlobalStats::new();
        assert_eq!(global_stats.annihilations(), 0);
        assert_eq!(global_stats.commutations(), 0);
        assert_eq!(global_stats.binds(), 0);
        assert_eq!(global_stats.connects(), 0);

        let mut stats = LocalStats::new();
        stats.inc_anni_era_era();
        stats.inc_anni_app_app();
        stats.inc_anni_lam_lam();
        stats.inc_anni_dup_dup();
        stats.inc_comm_dup_dup();
        stats.inc_comm_era_app();
        stats.inc_comm_era_lam();
        stats.inc_commute_era_dup();
        stats.inc_comm_app_lam();
        stats.inc_comm_app_dup();
        stats.inc_comm_lam_dup();
        stats.inc_binds();
        stats.inc_connects();

        global_stats.update(stats);
        assert_eq!(global_stats.annihilations(), 4);
        assert_eq!(global_stats.commutations(), 7);
        assert_eq!(global_stats.binds(), 1);
        assert_eq!(global_stats.connects(), 1);
        assert_eq!(global_stats.allocs(), 1);

        println!("{}", global_stats);
    }
}
