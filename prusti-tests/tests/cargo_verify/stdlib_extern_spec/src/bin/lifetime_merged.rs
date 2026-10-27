use prusti_contracts::*;

// The generated call accepts shortened borrows, but the target permits two
// independent lifetimes. Its contract must preserve that distinction.
#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn first<'a>(first: &'a u32, second: &'a u32) -> &'a u32;
// expected-note: Lifetime relationships must match method 'target_api::first' exactly

fn main() {}
