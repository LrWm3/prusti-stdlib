use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    let previous = map.remove(&2);
    prusti_assert!(matches!(previous, None)); // expected-error: assertion might not hold
}
