use prusti_contracts::*;

#[requires(values.len() > 0)]
#[requires(*mut_slice_referent_value(&values, Int::from(0)) == 1)]
fn observe(values: &mut [&mut u32]) {
    let elements = ghost! { Seq::from_slice(&*values) };
    let initial = ghost! { sequence_resource_state(&*elements, Int::from(0)) };
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == *initial);
    prusti_assert!(*mut_slice_referent_value(&values, Int::from(0)) == 1);
}

#[trusted]
fn main() {
    let mut value = 1;
    observe(&mut [&mut value]);
    assert_eq!(value, 1);
}
