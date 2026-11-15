use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let reference = &mut value;
    let elements = ghost! { Seq::single_ref(&reference).append(Seq::single_ref(&reference)) };
    let initial = ghost! { sequence_resource_state(&*elements, Int::from(0)) };
    *reference = 7;
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == sequence_resource_state(&*elements, Int::from(1)));
    prusti_assert!(sequence_resource_state(&*elements, Int::from(1)) != *initial);
}
