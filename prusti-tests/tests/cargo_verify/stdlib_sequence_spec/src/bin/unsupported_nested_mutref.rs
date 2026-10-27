use prusti_contracts::*;

// Isolate pure observation of a nested mutable reference. There is no impure
// body reading through it, so this tests the snapshot limitation alone.
#[requires(forall(|value: &mut &mut i32| **value == 0))] // expected-unsupported: cannot encode precondition: dereference of the mutable reference `&mut &mut i32` in pure code
fn nested_observation() {}
fn main() {}
