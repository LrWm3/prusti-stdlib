use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    map.insert(3, 9);
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 9); // expected-error: assertion might not hold
}
