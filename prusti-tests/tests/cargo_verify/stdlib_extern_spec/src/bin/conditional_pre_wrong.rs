use prusti_contracts::*;
use stdlib_specs::conditional_pre;

fn main() {
    let marker = 1usize;
    let result = conditional_pre(&marker, 0);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}
