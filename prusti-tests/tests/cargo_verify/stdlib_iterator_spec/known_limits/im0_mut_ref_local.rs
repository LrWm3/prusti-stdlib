use prusti_contracts::*;

#[requires(values.len() == 1)]
fn visit(values: &mut [&mut u32]) {
    let value = &mut *values[0];
    *value = 7;
    prusti_assert!(*value == 7);
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut [&mut value]);
    assert_eq!(value, 7);
}
