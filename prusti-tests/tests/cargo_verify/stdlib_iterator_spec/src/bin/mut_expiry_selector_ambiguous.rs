use prusti_contracts::*;
#[after_expiry(value => true)]
fn identity<'a, 'b>(value: (&'a mut u32, &'b mut u32)) -> (&'a mut u32, &'b mut u32) { value } // expected-unsupported: cannot encode method `identity`: expiry selector's input type must contain a unique borrowed lifetime
fn main() {}
