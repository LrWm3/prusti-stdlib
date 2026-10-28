use key_specs::{Key, Projection};
use prusti_contracts::*;

fn wrong() {
    let first = Key { id: 2, ignored: 1 };
    let colliding = Key { id: 3, ignored: 1 };
    prusti_assert!(first.hash_model() == colliding.hash_model());
    prusti_assert!(first.project() == colliding.project()); // expected-error: assertion might not hold
}
fn main() {}
