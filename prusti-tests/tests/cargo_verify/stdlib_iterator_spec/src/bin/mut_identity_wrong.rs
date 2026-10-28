use prusti_contracts::*;

fn wrong(first: &mut u32, second: &mut u32) {
    prusti_assert!(mut_ref_same_address(&first, &second)); // expected-error: assertion might not hold
}
fn main() {}
