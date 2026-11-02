use prusti_contracts::*;

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut values = Vec::new();
    values.push(&mut first);
    let selected = &mut values[..];
    let previous = core::mem::replace(&mut selected[0], &mut second);
    *previous = 7;
    *values[0] = 11;
    let previous_value = *previous;
    let current_value = *values[0];
    prusti_assert!(previous_value == 7);
    prusti_assert!(current_value == 11);
}
