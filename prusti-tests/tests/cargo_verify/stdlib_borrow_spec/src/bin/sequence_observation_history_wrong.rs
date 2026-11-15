use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let reference = &mut value;
    let elements = ghost! { Seq::single_ref(&reference) };
    let initial = ghost! { sequence_resource_state(&*elements, Int::from(0)) };
    prusti_assert!(*initial == resource_state(&reference));
    *reference = 7;
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == *initial); // expected-error: assertion might not hold
}
