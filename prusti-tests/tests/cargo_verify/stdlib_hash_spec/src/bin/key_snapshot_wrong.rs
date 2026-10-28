use key_specs::Key;
use prusti_contracts::*;

fn wrong() {
    let first = Key { id: 2, ignored: 1 };
    let equal = Key { id: 2, ignored: 9 };
    prusti_assert!(first === equal); // expected-error: assertion might not hold
}
fn main() {}
