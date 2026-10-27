use prusti_contracts::*;

#[requires(*value == 0)]
fn wrong(value: &mut usize) {
    loop {
        body_invariant!(*value == 0); // expected-error: loop invariant might not be preserved
        *value = 1;
    }
}

fn main() {}
