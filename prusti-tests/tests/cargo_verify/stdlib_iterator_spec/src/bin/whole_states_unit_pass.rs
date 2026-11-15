use prusti_contracts::*;

fn observe(values: Seq<()>) {
    let states = ghost! { sequence_resource_states(&values) };
    prusti_assert!((*states).len() == values.len());
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < values.len()) ==>
        *(*states)[j] == sequence_resource_state(&values, j)));
}
fn main() {}
