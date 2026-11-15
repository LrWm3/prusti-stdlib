use prusti_contracts::*;
fn read(value: &&mut u32) {
    let _state = ghost! { resource_state(value) }; // expected-error: insufficient permission to observe resource state
}
fn main() {}
