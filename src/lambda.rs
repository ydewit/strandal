use crate::strandal::{cell::CellRef, equation::VarPort, net::NetBuilder, term::TermRef};

/// M0 multiplexor
pub fn m_0(net: &mut impl NetBuilder) -> CellRef {
    let era = net.era();
    return era;
}

/// M1 multiplexor
pub fn m_1(net: &mut impl NetBuilder) -> (VarPort, VarPort) {
    let (root, x0) = net.var();
    return (root, x0);
}

pub fn m_2(net: &mut impl NetBuilder) -> (TermRef, [VarPort; 2]) {
    let aux_0 = net.var();
    let aux_1 = net.var();
    let ctr = net.ctr(aux_0.0, aux_1.0);
    let (root, aux) = m_1(net);
    net.bind(aux, ctr);
    return (root.into(), [aux_0.1, aux_1.1]);
}

pub fn m_3(net: &mut impl NetBuilder) -> (TermRef, [VarPort; 3]) {
    let (root, [aux_0, aux_1]) = m_2(net);
    let new_aux_1 = net.var();
    let aux_2 = net.var();
    let ctr = net.ctr(new_aux_1.0, aux_2.0);
    return (root.into(), [aux_0, new_aux_1.1, aux_2.1]);
}

pub fn id(b: &mut impl NetBuilder) -> VarPort {
    let id_var = b.var();
    let lam = b.ctr(id_var.0, id_var.1);

    let result = b.var();
    b.bind(result.0, lam);
    return result.1;
}

pub fn dup(b: &mut impl NetBuilder) -> VarPort {
    let var1 = b.var();
    let var2 = b.var();

    let app_ref = b.ctr(var2.0, var1.0);
    let dup_ref = b.dup(var2.1, app_ref);
    let lam_ref = b.ctr(dup_ref, var1.1);

    let result = b.var();
    b.bind(result.0, lam_ref);
    return result.1;
}
