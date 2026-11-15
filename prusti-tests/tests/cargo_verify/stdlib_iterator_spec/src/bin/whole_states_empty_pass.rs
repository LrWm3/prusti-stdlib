use prusti_contracts::*;

fn observe() {
    let states = ghost! { sequence_resource_states(&Seq::<u32>::new()) };
    prusti_assert!((*states).len() == Int::from(0));
}
fn main() { observe(); }
