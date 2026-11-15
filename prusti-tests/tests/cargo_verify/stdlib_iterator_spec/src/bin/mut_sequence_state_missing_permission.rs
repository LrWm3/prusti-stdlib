use prusti_contracts::*;

#[requires(values.len() > Int::from(0))]
fn observe<T>(values: Seq<T>) {
    let state = ghost! { sequence_resource_state(&values, Int::from(0)) }; // expected-error: insufficient permission to observe sequence resource state
}

fn main() {}
