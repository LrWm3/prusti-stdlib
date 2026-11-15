use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let loan = &mut value;
    *loan = 7;
    prusti_assert!(value == 4); // expected-error: assertion might not hold
}
