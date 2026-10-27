use prusti_contracts::*;

#[requires(value.len() == 2)]
fn wrong(value: &[i32]) {
    prusti_assert!(*Seq::from_slice(value)[1] == 99); // expected-error: assertion might not hold
}

fn main() {}
