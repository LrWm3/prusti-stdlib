use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let parent = &mut value;
    let child = &mut *parent;
    *child = 7;
    *parent = 9;
    prusti_assert!(value == 7); // expected-error: assertion might not hold
}
