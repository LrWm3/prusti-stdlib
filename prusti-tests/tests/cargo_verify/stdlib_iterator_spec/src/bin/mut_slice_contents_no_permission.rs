use prusti_contracts::*;

fn no_permission(values: &&mut [u32]) {
    let contents = ghost! { mut_slice_contents(values) }; // expected-error: insufficient permission to read mutable slice contents
    prusti_assert!((*contents).len() >= Int::from(0));
}

fn main() {}
