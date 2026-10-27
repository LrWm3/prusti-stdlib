use prusti_contracts::*;
use stdlib_specs::{RefinedFinite, RefinedOnce, refined_advance};
fn generic<I: RefinedFinite>(it: &mut I) {
    let before = ghost! { it.remaining() };
    let result = refined_advance(it);
    match result {
        Some(value) => {
            prusti_assert!(value === *(*before)[0]);
        }
        None => {
            prusti_assert!((*before).len() == Int::from(0));
        }
    }
}
fn main() {
    let mut it = RefinedOnce {
        value: 7,
        available: true,
    };
    assert!(matches!(refined_advance(&mut it), Some(7)));
    assert!(matches!(refined_advance(&mut it), None));
    generic(&mut it);
}
