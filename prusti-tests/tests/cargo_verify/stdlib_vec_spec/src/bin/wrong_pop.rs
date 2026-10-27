use prusti_contracts::*;

fn main() {
    let mut values = Vec::new();
    values.push(5);
    values.push(9);
    let last = values.pop().unwrap();
    prusti_assert!(last == 5); // expected-error: assertion might not hold
}
