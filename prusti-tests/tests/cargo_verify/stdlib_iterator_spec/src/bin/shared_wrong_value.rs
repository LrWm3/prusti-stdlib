use iterator_specs::advance;
use prusti_contracts::*;

fn main() {
    let values = [11usize, 22];
    let mut iterator = values.iter();
    let value = advance(&mut iterator).unwrap();
    prusti_assert!(*value == 22); // expected-error: assertion might not hold
}
