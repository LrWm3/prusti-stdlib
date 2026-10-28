use prusti_contracts::*;

fn wrong(values: &mut [u32]) {
    for value in &mut *values {
        *value = 7;
    }
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
