use prusti_contracts::*;

fn main() {
    let mut values = Vec::new();
    values.push(11usize);
    values.push(22usize);
    let mut iterator = values.into_iter();
    let value = iterator.next().unwrap();
    prusti_assert!(value == 22); // expected-error: assertion might not hold
}
