use core::ops::RangeInclusive;
use prusti_contracts::*;

// Equal endpoints alone do not establish freshness: an exhausted inclusive
// iterator can also have equal endpoints, and then indexes an empty slice.
#[requires(values.len() == 8)]
#[requires(*range.start() == 3)]
#[requires(*range.end() == 3)]
fn observe(values: &[usize], range: RangeInclusive<usize>) {
    let part = values.get(range);
    let singleton = match part {
        Some(value) => value.len() == 1,
        None => false,
    };
    prusti_assert!(singleton); // expected-error: assertion might not hold
}

fn main() {}
