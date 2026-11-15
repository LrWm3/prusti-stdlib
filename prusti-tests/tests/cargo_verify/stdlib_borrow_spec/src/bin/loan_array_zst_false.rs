use prusti_contracts::*;
struct Empty;
fn main() {
    let mut values = [Empty, Empty, Empty];
    let selected = &mut values[1];
    let previous = core::mem::replace(selected, Empty);
    let _returned = previous;
    prusti_assert!(false); // expected-error: assertion might not hold
}
