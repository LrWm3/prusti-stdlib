use prusti_contracts::*;

#[pure]
fn identity<'a, 'b>(value: &'a &'b mut u32) -> &'a &'b mut u32 { value }

#[after_expiry(forall(|i: usize| i < 1 ==> **identity(&value) != before_expiry(**identity(&result))))] // expected-error: pledge postcondition might not hold
fn borrow(value: &mut u32) -> &mut u32 { value }

fn main() {}
