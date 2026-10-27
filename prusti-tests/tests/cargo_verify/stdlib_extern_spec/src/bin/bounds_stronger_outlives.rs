use prusti_contracts::*;

#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn outlives<'a: 'b, 'b: 'a>(first: &'a u32, second: &'b u32) -> &'b u32;
// expected-note: External specification requires a bound not guaranteed by the target

fn main() {}
