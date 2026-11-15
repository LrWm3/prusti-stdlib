use prusti_contracts::*;

#[pure]
fn read(values: &&mut [u32]) -> Seq<u32> { mut_slice_contents(values) } // expected-error: insufficient permission to read mutable slice contents

fn no_permission(values: &&mut [u32]) {
    let contents = ghost! { read(values) };
    prusti_assert!((*contents).len() >= Int::from(0));
}

fn main() {}
