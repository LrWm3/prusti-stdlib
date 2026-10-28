use prusti_contracts::*;

fn wrong(values: &mut [()]) {
    for value in &mut *values {
        *value = ();
    }
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
