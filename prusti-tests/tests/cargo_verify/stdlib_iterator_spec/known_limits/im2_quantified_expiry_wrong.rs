use prusti_contracts::*;

#[after_expiry(forall(|i: usize| i < 1 ==> *value != before_expiry(*result)))]
fn wrong(value: &mut u32) -> &mut u32 { value }

fn main() {}
