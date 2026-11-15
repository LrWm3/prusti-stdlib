use prusti_contracts::*;
fn read(value: &mut u32) {
    let _state = resource_state(&value); // expected-unsupported: cannot encode method body `read`: `prusti_contracts` builtin ResourceState cannot be used in executable code
}
#[trusted]
fn main() {}
