🚧 **Note to Visitors:**
This repository houses a Interaction Combinators engine, which is currently a work in progress (WIP). We're actively developing and refining the engine, so expect changes and enhancements. Your insights and contributions are welcome during this phase!

## Implementation Features

This Interaction Combinators engine introduces a few optimizations worth mentioning:

- [X] **Structured concurrency**: Leverages the power of the [Rayon crate](https://docs.rs/rayon/latest/rayon/) to implement structured concurrency, allowing us to manage task parallelism with fine-grained control and robust error handling. This means our computations are not only fast but also resilient and maintainable.
- [X] **Zero-cost erase cells**: Erase cells operate without additional allocation overhead, thanks to their unboxed representation.
- [X] **Optimized connections**: Self auxiliary port connections are directly established without the need for variable allocation. This optimization also extends to reductions by directly evaluating both sides as a direct equation.
- [X] **Efficient cell handling**: Cells exist transiently on the execution stack, only persisting in the store when explicitly assigned to a variable during reduction.
- [X] **Recycling of pointers**: Rather than deallocating, we recycle pointers to variables and cells, minimizing memory churn.
- [X] **Immutable Cells**: Cells are immutable and exist on the stack, while variables are mutable and passed by reference, ensuring thread safety during reduction.
- [X] **Local Statistics Gathering**: Each thread collects its own statistics to minimize contention, contributing to global statistics post-execution.
- [X] **Depth-first Reduction**: We prioritize depth of work within threads to avoid extraneous async task allocations and queuing, streamlining execution.

TODOs:
- [ ] Reusable definitions
- [ ] Parser
- [ ] Tests
- [ ] Implement labels
- [ ] Add examples
- [ ] Add benchmarks

Some known issues:
- When walking linked Var's during binds or connects, the engine ends up reading the Var from the Store a second time.

## Acknowledgements
I would like to express my gratitude to Victor Taelin and the [Higher Order Company](https://github.com/HigherOrderCO) (HoC) for their passionate work in the field of interaction nets and interaction combinators, which introduced me to these fascinating concepts.
