use prusti_contracts::*;
use stdlib_specs::conditional_replace;

fn main() {
    let marker = 1usize;
    let mut value = 4usize;
    let previous = conditional_replace(&mut value, &marker);
    prusti_assert!(previous == 9); // expected-error: assertion might not hold
}
