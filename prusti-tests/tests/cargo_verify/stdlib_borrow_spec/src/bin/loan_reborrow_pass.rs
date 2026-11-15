use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let parent = &mut value;
    let child = &mut *parent;
    let grandchild = &mut *child;
    *grandchild = 7;
    prusti_assert!(*child == 7);
    *child = 8;
    prusti_assert!(*parent == 8);
    *parent = 9;
    prusti_assert!(value == 9);
}
