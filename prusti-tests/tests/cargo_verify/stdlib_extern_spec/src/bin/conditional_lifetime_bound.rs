use prusti_contracts::*;
use stdlib_specs::conditional_lifetime_bound;

fn main() {
    let value = 1usize;
    let marker = ();
    let result = conditional_lifetime_bound(&value, &marker);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}
