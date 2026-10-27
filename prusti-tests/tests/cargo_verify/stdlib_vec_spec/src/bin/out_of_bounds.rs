use prusti_contracts::*;

fn wrong(values: &Vec<usize>) {
    let _value = values[values.len()]; // expected-error: precondition might not hold
}

fn main() {}
