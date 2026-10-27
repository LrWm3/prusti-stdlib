//! External contracts exported to the separately compiled proof clients.
use prusti_contracts::*;

// These verified wrappers consume bundled contracts and export their own
// contracts to a third crate. The focused runner verifies this crate explicitly.
#[ensures(result === value)]
pub fn unwrap_owned<T>(value: T) -> T {
    Some(value).unwrap()
}

#[ensures(result === *old(&*dest))]
#[ensures(*dest === src)]
pub fn replace_owned<T>(dest: &mut T, src: T) -> T {
    core::mem::replace(dest, src)
}

#[extern_spec(target_api)]
trait Identity {
    #[trusted]
    #[ensures(result === value)]
    fn identity(value: Self::Item) -> Self::Item;

    #[trusted]
    #[ensures(*result === *value)]
    fn borrow<'a>(value: &'a Self::Item) -> &'a Self::Item;
}

#[extern_spec(target_api)]
trait Relay<T> {
    #[trusted]
    #[ensures(result === value)]
    fn relay(value: T) -> T;
}

#[extern_spec(target_api)]
trait Bounded {
    #[trusted]
    #[requires(value <= 10)]
    #[ensures(result <= 10)]
    fn bounded(value: u32) -> u32;
}

#[extern_spec]
impl target_api::Bounded for target_api::Zero {
    #[trusted]
    #[requires(value <= 10)]
    #[ensures(result == 0)]
    fn bounded(value: u32) -> u32;
}

#[extern_spec(target_api)]
#[trusted]
#[ensures(result === value)]
fn copied<T: Copy + Clone>(value: T) -> T;

#[extern_spec(target_api)]
#[trusted]
#[ensures(result === value)]
fn static_value<T: 'static>(value: T) -> T;

#[extern_spec(target_api)]
#[trusted]
#[ensures(result === value)]
fn bounded_identity<'a, T: 'a>(value: T, marker: &'a ()) -> T;

#[extern_spec(target_api)]
trait Copier<T: Copy> {
    #[trusted]
    #[ensures(result === value)]
    fn copy(value: T) -> T;
}

#[extern_spec(target_api)]
#[trusted]
#[ensures(*result == *first)]
fn first<'left, 'right>(first: &'left u32, second: &'right u32) -> &'left u32;

#[extern_spec(target_api)]
trait Consume {
    #[trusted]
    #[ensures(result == 0)]
    fn consume(self) -> u32;
}

#[extern_spec(target_api)]
trait Observe {
    #[trusted]
    #[ensures(result == 0)]
    fn observe(&self) -> u32;
}
