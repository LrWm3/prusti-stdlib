use prusti_contracts::*;

#[requires(values.len() == 1)]
#[ensures(result.len() == old(values.len()))]
#[after_expiry(values => {
    let source = old(mut_slice_contents(&values));
    Seq::from_slice(&*values).len() == source.len()
})]
fn relay(values: &mut [u32]) -> &mut [u32] { values }

#[requires(values.len() == 1)]
fn visit(values: &mut [u32]) {
    let returned = relay(values);
    returned[0] = 7;
    let length = values.len();
    prusti_assert!(length == 1);
}

fn main() { visit(&mut [1]); }
