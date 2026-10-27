use prusti_contracts::*;

#[requires(values.len() == 8)]
#[requires(*range.start() == 3 && *range.end() == 3)]
#[rustfmt::skip]
fn arbitrary(values: &mut [usize], range: core::ops::RangeInclusive<usize>) {
    let part = values.get_mut(range);
    let singleton = match part {
        Some(value) => value.len() == 1,
        None => false,
    };
    prusti_assert!(singleton); // expected-error: assertion might not hold
}

fn main() {}
