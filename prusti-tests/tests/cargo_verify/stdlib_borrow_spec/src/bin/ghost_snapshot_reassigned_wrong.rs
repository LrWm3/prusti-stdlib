use prusti_contracts::*;

fn visit() {
    let mut value = ghost! { 0u32 };
    let mut count = 0usize;
    loop {
        body_invariant!(count <= 1);
        body_invariant!(count == 1 ==> *value == 1);
        if count == 1 { break; }
        value = ghost! { 1u32 };
        count += 1;
    }
    prusti_assert!(*value == 0); // expected-error: assertion might not hold
}

fn main() {}
