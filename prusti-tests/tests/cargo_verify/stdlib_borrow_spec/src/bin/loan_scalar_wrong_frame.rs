use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let frame = 11u32;
    let loan = &mut value;
    *loan = 7;
    prusti_assert!(frame == 7); // expected-error: assertion might not hold
}
