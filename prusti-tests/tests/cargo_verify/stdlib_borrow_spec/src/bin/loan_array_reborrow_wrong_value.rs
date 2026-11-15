use prusti_contracts::*;
fn main() {
    let mut values = [4u32, 11, 15];
    let parent = &mut values[1];
    let child = &mut *parent;
    *child = 7;
    let child_final = *child;
    *parent = 9;
    prusti_assert!(child_final == 7);
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
    prusti_assert!(values[0] == 4 && values[2] == 15);
}
