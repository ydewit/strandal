## Interaction Nets: Understanding Vars and Cells

In the realm of interaction nets, a **Var** plays a crucial role in facilitating interactions between Cells. Conceptually, a Var serves as a temporal decoupler for Cells. At a given moment in the future, a Cell will traverse the Var, initiating a new interaction or establishing a connection with another Cell's port.

### Configurations of Vars

A Var can manifest in several configurations:

1. **Bind**: `Bind(var, cell)`
   - The Var is prepared to hold a Cell. Once set, the Cell can traverse this Var in subsequent interactions.

2. **Connect**: `Connect(var_x, var_y)`
   - Represents a direct connection between two Vars. This configuration sets the stage for Cells to potentially interact or connect across these Vars in the future.

3. **Cell Reference**: `Cell(.., var)`
   - A Cell might reference a Var through one of its ports, indicating a potential future interaction through that port.

### Var States

Independently of the aforementioned configurations, a Var can exist in one of the following states:

1. **Set State**: `var(Cell(..))`
   - Indicates that a Cell has been communicated from one end of the Var and now awaits interaction from the other end.

2. **Unset State**: `var(nil)`
   - Signifies an empty Var, awaiting Cells from either side. In this state, no Cell has yet been set for potential interactions.
