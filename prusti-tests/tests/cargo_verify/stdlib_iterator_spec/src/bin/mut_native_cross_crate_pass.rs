use prusti_contracts::*;

#[requires(values.len() == 2 && values[0] == 1 && values[1] == 2)]
fn visit(values: &mut [u32]) {
    iterator_specs::set_first_native(values);
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 2);
}

#[trusted]
fn main() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 2]);
}
