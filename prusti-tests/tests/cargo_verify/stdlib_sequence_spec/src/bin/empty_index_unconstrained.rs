use prusti_contracts::*;

#[requires(value.is_empty())]
fn wrong(value: &[i32]) {
    prusti_assert!(*Seq::from_slice(value)[0usize] == 0); // expected-error: assertion might not hold
}

fn main() {}
