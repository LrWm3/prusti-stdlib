use prusti_contracts::*;
use std::collections::HashSet;

fn main() {
    let set = HashSet::<u32>::new();
    prusti_assert!(prusti_contracts::std_spec::hash::set_contents(&set).len() > Int::from(0)); // expected-error: assertion might not hold
}
