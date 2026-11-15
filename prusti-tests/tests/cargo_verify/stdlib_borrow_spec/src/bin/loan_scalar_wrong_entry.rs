use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let entry = value;
    let loan = &mut value;
    *loan = 7;
    prusti_assert!(entry == 7); // expected-error: assertion might not hold
}
