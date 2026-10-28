use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashSet;

fn empty<K: KeyProjection>() {
    let set = HashSet::<K>::new();
    prusti_assert!(prusti_contracts::std_spec::hash::set_contents(&set).len() == Int::from(0)); // expected-error: assertion might not hold
}

fn main() {}
