use key_specs::Guarded;
use prusti_contracts::*;

fn wrong<T: Guarded>(value: &T) {
    let result = value.apply(1);
    prusti_assert!(result == 1); // expected-error: assertion might not hold
}
fn main() {}
