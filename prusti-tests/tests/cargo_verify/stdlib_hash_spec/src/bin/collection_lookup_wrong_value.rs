use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    if let Some(value) = map.get(&2) {
        prusti_assert!(*value == 9); // expected-error: assertion might not hold
    }
}
