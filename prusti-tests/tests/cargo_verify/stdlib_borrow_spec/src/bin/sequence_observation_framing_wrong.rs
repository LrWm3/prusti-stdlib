use prusti_contracts::*;
fn main() {
    let mut first = 4u32;
    let mut second = 9u32;
    let a = &mut first;
    let b = &mut second;
    let saved = ghost! { Seq::single_ref(&a).append(Seq::single_ref(&b)) };
    let initial_second = ghost! { sequence_resource_state(&*saved, Int::from(1)) };
    *a = 7;
    prusti_assert!(sequence_resource_state(&*saved, Int::from(1)) == *initial_second);
    prusti_assert!(sequence_resource_state(&*saved, Int::from(0)) == *initial_second); // expected-error: assertion might not hold
}
