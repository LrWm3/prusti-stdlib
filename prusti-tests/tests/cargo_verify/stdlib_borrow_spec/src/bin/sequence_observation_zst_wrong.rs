use prusti_contracts::*;
fn main() {
    let mut value = ();
    let reference = &mut value;
    let elements = ghost! { Seq::single_ref(&reference).append(Seq::single_ref(&reference)) };
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == sequence_resource_state(&*elements, Int::from(1)));
    prusti_assert!(false); // expected-error: assertion might not hold
}
