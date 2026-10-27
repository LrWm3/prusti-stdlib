use prusti_contracts::*;

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    values[1] = 7;
    prusti_assert!(values[1] == 2); // expected-error: assertion might not hold
}
