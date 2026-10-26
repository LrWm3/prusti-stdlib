extern crate stdlib_specs;

use prusti_contracts::*;
use target_api::{Bounded, Consume, Copier, Identity, Observe, Owned, Relay, Zero};

struct Token {
    value: u32,
}

#[ensures(result === value)]
fn generic_identity<I: Identity>(value: I::Item) -> I::Item {
    I::identity(value)
}

#[ensures(*result === *value)]
fn generic_borrow<'a, I: Identity>(value: &'a I::Item) -> &'a I::Item {
    I::borrow(value)
}

#[requires(value <= 10)]
#[ensures(result <= 10)]
fn generic_bound<B: Bounded + ?Sized>(value: u32) -> u32 {
    B::bounded(value)
}

#[ensures(result === value)]
fn generic_relay<T, R: Relay<T>>(value: T) -> T {
    R::relay(value)
}

#[ensures(result == 0)]
fn generic_observe<T: Observe + ?Sized>(value: &T) -> u32 {
    value.observe()
}

struct Local;

#[refine_trait_spec]
impl Bounded for Local {
    #[requires(value <= 10)]
    #[ensures(result == 0)]
    fn bounded(value: u32) -> u32 {
        0
    }
}

fn main() {
    let token = generic_identity::<Owned<Token>>(Token { value: 17 });
    assert!(token.value == 17);
    let borrowed = generic_borrow::<Owned<Token>>(&token);
    assert!(borrowed.value == 17);
    let relayed = generic_relay::<Token, Owned<Token>>(token);
    assert!(relayed.value == 17);
    let scalar = 23;
    let reference = generic_identity::<Owned<&u32>>(&scalar);
    assert!(*reference == 23);
    assert!(generic_bound::<Local>(8) <= 10);
    assert!(Local::bounded(8) == 0);
    assert!(Zero::bounded(8) == 0);
    assert!(generic_bound::<[u32]>(8) <= 10);
    assert!(target_api::copied(19_u32) == 19);
    assert!(target_api::static_value(Token { value: 27 }).value == 27);
    assert!(Owned::<u32>::copy(31) == 31);
    assert!(*target_api::first(&3, &5) == 3);
    assert!(generic_observe(&Zero) == 0);
    assert!(Zero.consume() == 0);
}
