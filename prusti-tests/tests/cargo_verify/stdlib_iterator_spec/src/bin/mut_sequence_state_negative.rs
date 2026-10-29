use prusti_contracts::*;
fn observe(values: &mut [u32]) {
    let state = ghost! { sequence_resource_state(&Seq::from_slice(&*values), Int::from(-1)) }; // expected-error: the resource observation index may be out of bounds
}
fn main() {}
