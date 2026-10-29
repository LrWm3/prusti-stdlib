use prusti_contracts::*;
#[ensures(result === old(first))] // expected-error: postcondition might not hold
fn wrong<T>(first: T, second: T) -> T { second }
#[trusted]
fn main() {}
