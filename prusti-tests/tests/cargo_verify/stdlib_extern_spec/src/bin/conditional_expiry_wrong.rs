use prusti_contracts::*;
use stdlib_specs::conditional_borrow;

fn main() {
    let mut value = 1;
    *conditional_borrow(&mut value, 1usize) = 7;
    prusti_assert!(value == 8); // expected-error: assertion might not hold
}
