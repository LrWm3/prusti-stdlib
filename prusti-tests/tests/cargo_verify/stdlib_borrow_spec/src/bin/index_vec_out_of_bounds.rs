use prusti_contracts::*;

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values[1] = 7; // expected-error: precondition might not hold
}
