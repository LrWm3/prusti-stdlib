use prusti_contracts::*;

#[requires(!values.is_empty())]
fn wrong(values: &[usize]) {
    let mut i = 0usize;
    loop {
        body_invariant!(i <= 1);
        if i == 0 {
            prusti_assert!(values[0] == 0); // expected-error: assertion might not hold
            i += 1;
        } else {
            break;
        }
    }
}

fn main() {}
