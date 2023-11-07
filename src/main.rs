mod lambda;
mod strandal;

use lambda::{dup, id};
use strandal::net::Net;

use tracing::info;

use crate::{
    lambda::m_2,
    strandal::{net::NetBuilder, runtime::Runtime},
};

fn main() {
    tracing_subscriber::fmt::init();

    let mut net = Net::with_capacity(1 << 30);

    //
    let id = id(&mut net);
    let dup = dup(&mut net);
    let m2 = m_2(&mut net);
    net.eqn(id, dup);
    net.head(m2.0);

    //
    let r = net.var();
    let i1_var = net.var();
    let i1 = net.lam(i1_var.0, i1_var.1);
    let i2_var = net.var();
    let i2 = net.lam(i2_var.0, i2_var.1);
    let app = net.app(r.0, i2);
    net.head(r.1);
    net.eqn(i1, app);

    // info!("Initial Net: {}", net);

    let mut runtime = Runtime::new();
    runtime.eval(&mut net);
    // info!("Final Net: {}", net);
    info!("{}", runtime.stats);
}
