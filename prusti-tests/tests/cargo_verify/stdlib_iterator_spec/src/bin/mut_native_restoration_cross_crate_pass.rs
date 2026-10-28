use prusti_contracts::*;

#[ensures(forall(|i: usize| i < values.len() ==> values[i] == 7))]
fn visit(values: &mut [u32]) {
    iterator_specs::set_all_native(values);
    prusti_assert!(forall(|i: usize| i < values.len() ==> values[i] == 7));
}

#[trusted]
fn main() {
    let mut values = [1, 2, 3];
    visit(&mut values);
    assert_eq!(values, [7, 7, 7]);
}
