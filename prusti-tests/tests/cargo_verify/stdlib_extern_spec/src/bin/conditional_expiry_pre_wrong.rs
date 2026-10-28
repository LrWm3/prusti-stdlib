use prusti_contracts::*;
use stdlib_specs::conditional_borrow_with_pre;

fn main() {
    let mut value = 1;
    *conditional_borrow_with_pre(&mut value, 1usize, 0) = 7;
    prusti_assert!(value == 7); // expected-error: assertion might not hold
}
