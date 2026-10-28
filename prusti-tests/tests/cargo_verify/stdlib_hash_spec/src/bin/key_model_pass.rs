use key_specs::{equivalent, model_insert, Key, Projection};
use prusti_contracts::*;

#[trusted]
fn runtime_hash_collision() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let first = Key { id: 2, ignored: 1 };
    let equal = Key { id: 2, ignored: 9 };
    let colliding = Key { id: 3, ignored: 1 };
    let mut first_hash = DefaultHasher::new();
    let mut collision_hash = DefaultHasher::new();
    first.hash(&mut first_hash);
    colliding.hash(&mut collision_hash);
    assert!(first == equal);
    assert!(first != colliding);
    assert_eq!(first_hash.finish(), collision_hash.finish());
}

fn main() {
    let first = Key { id: 2, ignored: 1 };
    let equal = Key { id: 2, ignored: 9 };
    let colliding = Key { id: 3, ignored: 1 };
    prusti_assert!(first == equal);
    prusti_assert!(!(first === equal));
    prusti_assert!(first.project() == equal.project());
    prusti_assert!(equivalent(&first, &equal));
    prusti_assert!(!equivalent(&first, &colliding));
    prusti_assert!(first.project() != colliding.project());
    prusti_assert!(first.hash_model() == colliding.hash_model());

    let model = model_insert(Map::new(), &first, 10);
    prusti_assert!(model.contains(first.project()));
    prusti_assert!(*model[equal.project()] == 10);
    let model = model_insert(model, &colliding, 20);
    prusti_assert!(model.len() == Int::from(2));
    prusti_assert!(*model[first.project()] == 10);
    prusti_assert!(*model[colliding.project()] == 20);

    let integer_model = model_insert(Map::new(), &4u32, 7);
    prusti_assert!(*integer_model[4u32] == 7);
    runtime_hash_collision();
}
