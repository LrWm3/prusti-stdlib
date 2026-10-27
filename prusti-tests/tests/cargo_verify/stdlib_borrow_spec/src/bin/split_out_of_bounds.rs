use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3, 4];
    let _ = values.split_at_mut(5); // expected-error: precondition might not hold
}
