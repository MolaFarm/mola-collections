# Mola Collections

`Mola Collections` is a Rust library that provides various data structures. This crate is designed for `no_std` environments and works out of the box.

## Collections

| Module | Data Structure | Description |
| - | - | - |
| hash   | LockedMap | A sharded concurrent HashMap based on `RwLock`. |
| hash   | RcuMap | A sharded concurrent HashMap designed using the RCU model and containing no locks. |
| hash | FixedMap | A fixed size HashMap can be allocated at compile time. |