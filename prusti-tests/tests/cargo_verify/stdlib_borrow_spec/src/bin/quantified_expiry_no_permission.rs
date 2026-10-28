use prusti_contracts::*;

#[pure]
fn identity<'a, 'b>(value: &'a &'b mut u32) -> &'a &'b mut u32 { value }

#[after_expiry(forall(|i: usize| i < 1 ==> before_expiry(**identity(&value)) == 0))] // expected-error: insufficient permission to read through mutable reference
fn borrow<'a, 'b>(value: &'a mut u32, other: &'b mut u32) -> &'b mut u32 { other }

fn main() {}
