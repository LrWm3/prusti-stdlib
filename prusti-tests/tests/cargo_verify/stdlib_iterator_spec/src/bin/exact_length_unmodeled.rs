use prusti_contracts::*;

fn wrong<I: ExactSizeIterator>(iterator: &I) {
    let length = iterator.len();
    prusti_assert!(length == 0); // expected-error: assertion might not hold
}
fn main() {}
