use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let entry = value;
    let frame = 11u32;
    let loan = &mut value;
    *loan = 7;
    prusti_assert!(value == 7);
    prusti_assert!(entry == 4);
    prusti_assert!(frame == 11);
}
