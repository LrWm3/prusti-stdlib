use prusti_contracts::*;

// A higher-ranked callback coerces to one that only accepts static borrows.
#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn static_callback(value: for<'a> fn(&'a u32) -> &'a u32);
// expected-note: Parameter and return types must match method 'target_api::static_callback' exactly

fn main() {}
