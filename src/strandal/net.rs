use super::{
    store::Store,
    term::{Cell, Term, TermPtr},
    var::{Var, VarUse},
};

#[derive(Debug)]
pub struct Net {
    pub(crate) head: Vec<TermPtr>,
    pub(crate) body: Vec<(TermPtr, TermPtr)>,
    pub(crate) store: Store,
}

impl Net {
    pub fn new() -> Self {
        Net {
            head: Default::default(),
            body: Default::default(),
            store: Store::new(),
        }
    }
    pub fn with_capacity(capacity: u32) -> Self {
        Net {
            head: Default::default(),
            body: Default::default(),
            store: Store::with_capacity(capacity),
        }
    }
}

pub trait NetBuilder {
    fn head<T>(&mut self, term_ref: T)
    where
        T: Into<TermPtr>;

    fn var(&mut self) -> (VarUse, VarUse);

    fn lam<T1, T2>(&mut self, binding: T1, body: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>;

    fn app<T1, T2>(&mut self, result: T1, arg: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>;

    fn dup<T1, T2>(&mut self, left: T1, right: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>;

    fn era(&mut self) -> TermPtr;

    fn eqn<T1, T2>(&mut self, left: T1, right: T2)
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>;
}

impl NetBuilder for Net {
    fn head<T: Into<TermPtr>>(&mut self, term_ptr: T) {
        self.head.push(term_ptr.into());
    }

    fn var(&mut self) -> (VarUse, VarUse) {
        let var_ptr = self.store.alloc(Term::Var(Var::new()).into());
        let var_port_0 = VarUse::new(var_ptr);
        let var_port_1 = VarUse::new(var_ptr);
        (var_port_0, var_port_1)
    }

    fn lam<T1, T2>(&mut self, binding: T1, body: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>,
    {
        let lam = Cell::Lam((binding.into(), body.into()).into());
        let cell_ptr = self.store.alloc(Term::Cell(lam).into());
        TermPtr::Ptr(cell_ptr)
    }

    fn app<T1, T2>(&mut self, lam: T1, arg: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>,
    {
        let app = Cell::App((lam.into(), arg.into()).into());
        let cell_ptr = self.store.alloc(Term::Cell(app).into());
        TermPtr::Ptr(cell_ptr)
    }

    fn dup<T1, T2>(&mut self, left: T1, right: T2) -> TermPtr
    where
        T1: Into<TermPtr>,
        T2: Into<TermPtr>,
    {
        let dup = Cell::Dup((left.into(), right.into()).into(), None);
        let cell_ptr = self.store.alloc(Term::Cell(dup).into());
        TermPtr::Ptr(cell_ptr)
    }

    fn era(&mut self) -> TermPtr {
        TermPtr::Era
    }

    fn eqn<T1: Into<TermPtr>, T2: Into<TermPtr>>(&mut self, left: T1, right: T2) {
        self.body.push((left.into(), right.into()));
    }
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use crate::strandal::{
        net::{Net, NetBuilder},
        runtime::Runtime,
    };

    #[test]
    fn test_net() {
        let mut net = Net::new();
        let r = net.var();
        let i1_var = net.var();
        let i1 = net.lam(i1_var.0, i1_var.1);
        let i2_var = net.var();
        let i2 = net.lam(i2_var.0, i2_var.1);
        let app = net.app(r.0, i2);
        net.head(r.1);
        net.eqn(i1, app);

        let mut runtime = Runtime::new();
        runtime.eval(&mut net);

        info!("net: {}", runtime.stats);
    }
}
