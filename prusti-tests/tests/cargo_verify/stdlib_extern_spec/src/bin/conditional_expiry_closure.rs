use prusti_contracts::*;

#[requires(f |= |value: u32| [after_expiry(value == 0)])] // expected-parse: conditional expiry is not supported in closure specifications
fn unsupported<F: Fn(u32) -> u32>(f: F) {}

fn main() {}
