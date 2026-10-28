use prusti_contracts::*;
use stdlib_specs::{refined_advance, RefinedOnceRef};

fn wrong<T>(value: &T) {
    let mut iterator = RefinedOnceRef {
        value,
        available: true,
    };
    let first = refined_advance(&mut iterator);
    prusti_assert!(matches!(first, None)); // expected-error: assertion might not hold
}
fn main() {}
