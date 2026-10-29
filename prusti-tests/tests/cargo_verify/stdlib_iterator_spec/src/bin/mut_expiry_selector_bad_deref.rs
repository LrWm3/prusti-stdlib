use prusti_contracts::*;
#[after_expiry(**value => true)]
fn identity(value: &mut u32) -> &mut u32 { value } // expected-unsupported: cannot encode method `identity`: expiry selector dereferences a non-reference input type
fn main() {}
