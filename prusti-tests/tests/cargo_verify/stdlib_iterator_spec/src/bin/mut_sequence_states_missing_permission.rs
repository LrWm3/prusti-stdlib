use prusti_contracts::*;
#[requires(values.len() > Int::from(0))]
fn observe<T>(values: Seq<T>) {
    let states = ghost! { sequence_resource_states(&values) }; // expected-error: insufficient permission to observe sequence resource states
}
fn main() {}
