use prusti_contracts::*;
use stdlib_specs::conditional_borrow;

fn generic<M>(value: &mut usize, marker: M) {
    *conditional_borrow(value, marker) = 7;
    prusti_assert!(*value <= 7);
    prusti_assert!(*value == 7); // expected-error: assertion might not hold
}
fn main() {}
