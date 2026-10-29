use prusti_contracts::*;
use resource_specs::{identity, Owned};

fn main() {
    let output = identity(Owned { value: 7 });
    prusti_assert!(output.value == 7);
    let shared = identity(&output.value);
    prusti_assert!(*shared == 7);
}
