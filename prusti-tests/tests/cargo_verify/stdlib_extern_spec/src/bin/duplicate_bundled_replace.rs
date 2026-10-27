use prusti_contracts::*;

#[extern_spec(std::mem)] // expected-incorrect: duplicate specification for `std::mem::replace` from crate `duplicate_bundled_replace` and `prusti_contracts`
#[trusted]
fn replace<T>(dest: &mut T, src: T) -> T;

fn main() {}
