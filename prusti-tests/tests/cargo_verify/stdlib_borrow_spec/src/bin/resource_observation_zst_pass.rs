use prusti_contracts::*;
fn main() {
    let mut first = ();
    let mut second = ();
    let a = &mut first;
    let b = &mut second;
    prusti_assert!(resource_state(&a) == resource_state(&b));
}
