use prusti_contracts::*;

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut current = &mut first;
    let previous = core::mem::replace(&mut current, &mut second);
    *previous = 7;
    *current = 11;
    let previous_value = *previous;
    let current_value = *current;
    prusti_assert!(previous_value == 7);
    prusti_assert!(current_value == 9); // expected-error: assertion might not hold
}
