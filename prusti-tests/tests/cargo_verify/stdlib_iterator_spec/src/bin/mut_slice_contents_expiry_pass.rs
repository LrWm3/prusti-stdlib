use prusti_contracts::*;

#[requires(values.len() == 1)]
#[ensures(result.len() == old(values.len()))]
#[after_expiry(Seq::from_slice(&*values) == before_expiry(mut_slice_contents(&result)))]
fn borrow(values: &mut [u32]) -> &mut [u32] { values }

#[requires(values.len() == 1)]
fn visit(values: &mut [u32]) {
    let borrowed = borrow(values);
    borrowed[0] = 7;
    prusti_assert!(values[0] == 7);
}

fn main() { let mut values = [1]; visit(&mut values); }
