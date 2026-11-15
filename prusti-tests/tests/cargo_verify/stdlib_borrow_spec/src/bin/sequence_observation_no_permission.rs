use prusti_contracts::*;
#[requires(values.len() > Int::from(0))]
fn read(values: Seq<&mut u32>) {
    let state = ghost! { sequence_resource_state(&values, Int::from(0)) }; // expected-error: insufficient permission to observe sequence resource state
}
fn main() {}
