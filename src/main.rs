mod lambda;
mod strandal;

use lambda::{dup, id};
use strandal::net::Net;

use tracing::{debug, info};

use crate::{
    lambda::m_2,
    strandal::{net::NetBuilder, runtime::Runtime},
};

fn main() {
    tracing_subscriber::fmt::init();

    let mut net = Net::new(1 << 30);
    let id_ptr = id(&mut net);
    let dup_ptr = dup(&mut net);
    let m2_ptr = m_2(&mut net);
    net.connect(id_ptr, dup_ptr);
    net.head(m2_ptr.0);

    info!("Initial Net: {}", net);

    let mut runtime = Runtime::new();
    runtime.eval(&mut net);

    for term in net.store.iter().enumerate() {
        debug!("Heap: {} -> {:?}", term.0, term.1);
    }

    info!("Final Net: {}", net);
    info!("Redexes: {}", runtime.redexes());
    info!("Binds: {}", runtime.binds());
    info!("Connects: {}", runtime.connects());
    info!("Annihilations: {}", runtime.annihilations());
    info!("Commutations: {}", runtime.commutations());
    info!("Erasures: {}", runtime.erasures());
}
