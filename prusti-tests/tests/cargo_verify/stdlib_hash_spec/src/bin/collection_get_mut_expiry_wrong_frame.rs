use prusti_contracts::*;
use std::collections::HashMap;
fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    map.insert(3, 9);
    if let Some(value) = map.get_mut(&2) {
        *value = 10;
    }
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[3] == 10); // expected-error: assertion might not hold
}
