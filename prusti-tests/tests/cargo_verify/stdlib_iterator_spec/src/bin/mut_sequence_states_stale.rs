use prusti_contracts::*;

#[requires(values.len() > 0)]
#[requires(*mut_slice_referent_value(&values, Int::from(0)) == 1)]
fn visit(values: &mut [&mut u32]) {
    let elements = ghost! { Seq::from_slice(&*values) };
    let initial = ghost! { sequence_resource_states(&*elements) };
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == *(*initial)[Int::from(0)]);
    *values[0] = 7;
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == *(*initial)[Int::from(0)]); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut [&mut value]);
    assert_eq!(value, 7);
}
