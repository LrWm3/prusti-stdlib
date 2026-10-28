use key_specs::{equivalent, Key, Projection};
use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;

fn laws(a: &Key, b: &Key, c: &Key) {
    prusti_assert!(a == a);
    prusti_assert!((a == b) == (b == a));
    prusti_assert!((a == b && b == c) ==> a == c);
    prusti_assert!((a == b) == (a.project() == b.project()));
    prusti_assert!((a == b) == (a.key_model() == b.key_model()));
    prusti_assert!((a == b) ==> a.hash_model() == b.hash_model());
}

fn integer_laws(a: &u32, b: &u32) {
    prusti_assert!(equivalent(a, b) == (*a == *b));
    prusti_assert!((a.project() == b.project()) == (*a == *b));
    prusti_assert!((a.key_model() == b.key_model()) == (*a == *b));
}

fn main() {
    let a = Key { id: 2, ignored: 1 };
    let b = Key { id: 2, ignored: 9 };
    let c = Key { id: 3, ignored: 1 };
    laws(&a, &b, &c);
    integer_laws(&2, &3);
}
