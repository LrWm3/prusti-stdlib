use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong(values: &mut [u32]) {
    let contents = ghost! { mut_slice_contents(&values) };
    prusti_assert!((*contents)[0] == (*contents)[1]); // expected-error: assertion might not hold
}

fn main() {}
