use prusti_contracts::*;
use stdlib_specs::RefinedFinite;
fn wrong<I: RefinedFinite>(it: &mut I) {
    let before = ghost! { it.remaining() };
    if let Some(_) = it.advance() {
        prusti_assert!((*before).len() == Int::from(0)); // expected-error: assertion might not hold
    }
}
fn main() {}
