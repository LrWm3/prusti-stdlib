use prusti_contracts::*;
#[pure]
fn identity<'a, 'b>(value: &'a &'b mut u32) -> &'a &'b mut u32 { value }
#[after_expiry(before_expiry(forall(|i: usize| i < 1 ==> **identity(&result) == 1)))] // expected-unsupported: cannot encode pledge rhs: specification closures inside before_expiry are not supported
fn borrow(value: &mut u32) -> &mut u32 { value }
fn main() {}
