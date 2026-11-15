use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let parent = &mut value;
    let first = &mut *parent;
    *first = 7;
    let first_final = *first;
    *parent = 9;
    let second = &mut *parent;
    prusti_assert!(*second == 9);
    *second = 11;
    prusti_assert!(*parent == 11);
    prusti_assert!(value == 11);
    prusti_assert!(first_final == 7);
}
