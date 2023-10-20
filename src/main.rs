mod icomb;

use icomb::{WirePtr, net::{Net, Cell, Equation}};
use tracing::info;

use crate::icomb::runtime::Runtime;

fn id(net: &mut Net) -> WirePtr {
    let root_wire = net.heap.alloc_wire();
    let id_wire = net.heap.alloc_wire();
    let lam_ptr = net.heap.alloc_cell();
    net.heap
        .set_cell(lam_ptr, Cell::Ctr(id_wire.into(), id_wire.into()));
    net.body.push(Equation::Bind {
        wire_ptr: root_wire,
        cell_ptr: lam_ptr,
    });
    return root_wire;
}

fn dup(net: &mut Net) -> WirePtr {
    let root_wire = net.heap.alloc_wire();

    let lam_ptr = net.heap.alloc_cell();
    let dup_ptr = net.heap.alloc_cell();
    let app_ptr = net.heap.alloc_cell();

    let wire1 = net.heap.alloc_wire();
    let wire2 = net.heap.alloc_wire();

    net.heap
        .set_cell(lam_ptr, Cell::Ctr(dup_ptr.into(), wire1.into()));
    net.heap
        .set_cell(dup_ptr, Cell::Ctr(wire2.into(), app_ptr.into()));
    net.heap
        .set_cell(app_ptr, Cell::Ctr(wire2.into(), wire1.into()));

    net.body.push(Equation::Bind {
        wire_ptr: root_wire,
        cell_ptr: lam_ptr,
    });
    return root_wire;
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut net = Net::new(1 << 30);
    let id_ptr = id(&mut net);
    let dup_ptr = dup(&mut net);

    net.body.push(Equation::Connect {
        left_ptr: id_ptr,
        right_ptr: dup_ptr,
    });

    info!("Initial Net: {:?}", net);

    let mut runtime = Runtime::new();
    runtime.eval(&mut net);

    for term in net.heap.iter().enumerate() {
        info!("Heap: {} -> {:?}", term.0, term.1);
    }

    info!("Final Net: {:?}", net);
    info!("Redexes: {}", runtime.redexes());
    info!("Binds: {}", runtime.binds());
    info!("Connects: {}", runtime.connects());
    info!("Annihilations: {}", runtime.annihilations());
    info!("Commutations: {}", runtime.commutations());
    info!("Erasures: {}", runtime.erasures());
}
