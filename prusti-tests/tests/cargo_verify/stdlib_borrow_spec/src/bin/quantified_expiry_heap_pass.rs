use prusti_contracts::*;

#[pure]
fn identity<'a, 'b>(value: &'a &'b mut u32) -> &'a &'b mut u32 { value }

#[pure]
fn marker(i: usize) -> usize { i }

#[after_expiry(forall(|i: usize| marker(i) == 0 ==> **identity(&value) == before_expiry(**identity(&result)), triggers = [(marker(i),)]))]
fn borrow(value: &mut u32) -> &mut u32 { value }

fn visit(value: &mut u32) {
    let borrowed = borrow(value);
    *borrowed = 7;
    prusti_assert!(marker(0) == 0);
    prusti_assert!(*value == 7);
}

fn main() {}
