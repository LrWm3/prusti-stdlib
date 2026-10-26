use prusti_contracts::*;

#[extern_spec(std::mem)] // expected-incorrect: Invalid external specification
#[trusted]
fn replace<T: 'static>(dest: &mut T, src: T) -> T;
// expected-note: External specification requires a bound not guaranteed by the target

fn main() {}
