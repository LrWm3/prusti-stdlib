use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let reference = &mut value;
    let saved = ghost! { Seq::single_ref(&reference) };
    *reference = 7;
    let seven = ghost! { sequence_resource_state(&*saved, Int::from(0)) };
    *reference = 9;
    prusti_assert!(sequence_resource_state(&*saved, Int::from(0)) != *seven);
    prusti_assert!(value == 9);
}
