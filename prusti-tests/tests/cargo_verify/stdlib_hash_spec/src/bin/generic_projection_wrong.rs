use key_specs::Projection;
use prusti_contracts::*;

fn wrong<K: Projection<Model = u32>>(key: &K) {
    prusti_assert!(key.project() == 0); // expected-error: assertion might not hold
}
fn main() {}
