use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3];
    values[1] = 7;
    prusti_assert!(values[1] == 2); // expected-error: assertion might not hold
}
