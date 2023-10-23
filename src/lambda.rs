use crate::icomb::net::{Net, cell::CellPtr, equation::Port, term::TermPtr};

/// M0 multiplexor
pub fn m_0(net: &mut Net) -> CellPtr {
    let era = net.era();
    return era;
}

/// M1 multiplexor
pub fn m_1(net: &mut Net) -> (Port, Port) {
    let (root, x0) = net.var();
    return (root, x0);
}

pub fn m_2(net: &mut Net) -> (TermPtr, [Port; 2]) {
    let aux_0 = net.var();
    let aux_1 = net.var();
    let ctr = net.ctr(aux_0.0, aux_1.0);
    let (root, aux) = m_1(net);
    net.bind(aux, ctr);
    return (root.into(), [aux_0.1, aux_1.1]);
}

pub fn m_3(net: &mut Net) -> (TermPtr, [Port; 3]) {
    let (root, [aux_0, aux_1]) = m_2(net);
    let new_aux_1 = net.var();
    let aux_2 = net.var();
    let ctr = net.ctr(new_aux_1.0, aux_2.0);
    return (root.into(), [aux_0, new_aux_1.1, aux_2.1]);
}

pub fn id(b: &mut Net) -> Port {
    let id_var = b.var();
    let lam = b.ctr(id_var.0, id_var.1);
    return b.free(lam);
}

pub fn dup(b: &mut Net) -> Port {
    let var1 = b.var();
    let var2 = b.var();

    let app_ptr = b.ctr(var2.0, var1.0);
    let dup_ptr = b.dup(var2.1, app_ptr);
    let lam_ptr = b.ctr(dup_ptr, var1.1);

    return b.free(lam_ptr);
}
