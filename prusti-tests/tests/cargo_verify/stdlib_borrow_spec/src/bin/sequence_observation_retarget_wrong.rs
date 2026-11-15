use prusti_contracts::*;
fn main() {
    let mut first = 4u32;
    let mut second = 9u32;
    let mut reference = &mut first;
    let saved = ghost! { Seq::single_ref(&reference) };
    reference = &mut second;
    *reference = 7;
    prusti_assert!(sequence_resource_state(&*saved, Int::from(0)) == resource_state(&reference)); // expected-error: assertion might not hold
    prusti_assert!(first == 4);
    prusti_assert!(second == 7);
}
