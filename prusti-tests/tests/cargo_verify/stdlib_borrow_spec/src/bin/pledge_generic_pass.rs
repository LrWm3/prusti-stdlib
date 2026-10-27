use prusti_contracts::*;

fn main() {
    let mut value = 4usize;
    let borrowed = borrow_specs::identity_with_marker(&mut value, 7usize);
    assert!(*borrowed == 4);
    *borrowed = 9;
    assert!(value == 9);
}
