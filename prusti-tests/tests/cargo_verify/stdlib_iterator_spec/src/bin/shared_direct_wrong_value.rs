use prusti_contracts::*;

fn main() {
    let values = [11usize, 22];
    let slice: &[usize] = &values;
    let mut iterator = slice.iter();
    let value = iterator.next().unwrap();
    prusti_assert!(*value == 22); // expected-error: assertion might not hold
}
