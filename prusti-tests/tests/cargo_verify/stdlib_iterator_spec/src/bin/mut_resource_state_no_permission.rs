use prusti_contracts::*;
fn read<T>(value: &T) {
    let _state = ghost! { resource_state(value) }; // expected-error: insufficient permission to observe resource state
}
#[trusted]
fn main() {}
