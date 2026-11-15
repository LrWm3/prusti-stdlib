use prusti_contracts::*;
fn main() {
    let elements = ghost! { Seq::single(4u32) };
    prusti_assert!(sequence_resource_state(&*elements, Int::from(0)) == resource_state(&4u32));
}
