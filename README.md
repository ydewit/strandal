



## Interaction Nets: Understanding Wires and Cells

In the realm of interaction nets, a **Wire** plays a crucial role in facilitating interactions between Cells. Conceptually, a Wire serves as a temporal decoupler for Cells. At a given moment in the future, a Cell will traverse the Wire, initiating a new interaction or establishing a connection with another Cell's port.

### Configurations of Wires

A Wire can manifest in several configurations:

1. **Bind**: `Bind(wire, cell)`
   - The Wire is prepared to hold a Cell. Once set, the Cell can traverse this Wire in subsequent interactions.

2. **Connect**: `Connect(wire_x, wire_y)`
   - Represents a direct connection between two Wires. This configuration sets the stage for Cells to potentially interact or connect across these Wires in the future.

3. **Cell Reference**: `Cell(.., wire)`
   - A Cell might reference a Wire through one of its ports, indicating a potential future interaction through that port.

### Wire States

Independently of the aforementioned configurations, a Wire can exist in one of the following states:

1. **Set State**: `wire(Cell(..))`
   - Indicates that a Cell has been communicated from one end of the Wire and now awaits interaction from the other end.

2. **Unset State**: `wire(nil)`
   - Signifies an empty Wire, awaiting Cells from either side. In this state, no Cell has yet been set for potential interactions.


