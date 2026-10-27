use prusti_contracts::*;

#[requires(n < usize::MAX)]
fn changed(mut n: usize) {
    n += 1;
    prusti_assert!(n == old(n)); // expected-error: assertion might not hold
}

fn main() {}
