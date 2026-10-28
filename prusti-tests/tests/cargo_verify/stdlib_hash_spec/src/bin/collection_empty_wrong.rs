use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let map = HashMap::<u32, u32>::new();
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&map).len() > Int::from(0)); // expected-error: assertion might not hold
}
