use prusti_contracts::*;
#[after_expiry(value => Ghost::new_ref(&*value) == before_expiry(Ghost::new_ref(&*result)))]
#[after_expiry(*value => **value == before_expiry(**result))] // expected-error: insufficient permission to read through mutable reference
fn nested<'a, 'b>(value: &'a mut &'b mut u32) -> &'a mut &'b mut u32 { value }
fn main() {}
