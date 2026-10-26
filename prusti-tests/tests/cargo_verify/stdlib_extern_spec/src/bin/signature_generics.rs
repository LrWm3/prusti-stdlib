use prusti_contracts::*;

#[extern_spec(std::mem)] // expected-incorrect: Invalid external specification
#[trusted]
fn replace(dest: &mut u32, src: u32) -> u32;
// expected-note: The number of type parameters must match the target method

fn main() {}
