use prusti_contracts::*;

fn main() {
    let mut value = 4usize;
    let borrowed = borrow_specs::identity_with_marker(&mut value, 7usize);
    *borrowed = 9;
    prusti_assert!(value == 4); // expected-error: assertion might not hold
}
