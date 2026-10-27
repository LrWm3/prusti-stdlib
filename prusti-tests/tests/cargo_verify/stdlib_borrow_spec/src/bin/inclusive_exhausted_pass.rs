use core::ops::RangeInclusive;
use prusti_contracts::*;

#[requires(values.len() == 8)]
#[requires(*range.start() == 3 && *range.end() == 3)]
#[requires(!(range === RangeInclusive::new(*range.start(), *range.end())))]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)))]
fn exhausted(values: &mut [usize], range: RangeInclusive<usize>) {
    match values.get_mut(range) {
        Some(part) => {
            prusti_assert!(part.len() == 0);
        }
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {}
