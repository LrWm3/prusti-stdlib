//! External contracts exported to the separately compiled proof clients.
use prusti_contracts::*;

pub trait ConditionalModel {}
impl ConditionalModel for usize {}
impl ConditionalModel for &'static u8 {}

pub trait StaticModel {}
impl<T: 'static> StaticModel for T {}

pub trait CopyRefModel {}
impl<T: Copy> CopyRefModel for &T {}

pub trait StaticRefModel {}
impl<T: 'static> StaticRefModel for &T {}

pub trait SameLifetimeModel {}
impl<'a> SameLifetimeModel for (&'a u8, &'a u8) {}

pub trait LifetimeModel<'a> {}
impl<'a> LifetimeModel<'a> for usize {}

#[trusted]
#[refine_spec(where Value: ConditionalModel, [ensures(result == 7)])]
pub fn conditional_observe<Value>(_value: &Value) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: StaticModel, [ensures(result == 7)])]
pub fn conditional_static<Value>(_value: &Value) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: CopyRefModel, [ensures(result == 7)])]
pub fn conditional_copy_ref<Value>(_value: &Value) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: StaticRefModel, [ensures(result == 7)])]
pub fn conditional_static_ref<Value>(_value: &Value) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: SameLifetimeModel, [ensures(result == 7)])]
pub fn conditional_same_lifetime<Value>(_value: &Value) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: LifetimeModel<'a>, [ensures(result == 7)])]
pub fn conditional_lifetime_bound<'a, Value>(_value: &Value, _marker: &'a ()) -> usize {
    7
}

#[trusted]
#[refine_spec(where Value: ConditionalModel, [
    requires(input > 0),
    ensures(result == 7),
])]
pub fn conditional_pre<Value>(_value: &Value, input: usize) -> usize {
    if input > 0 {
        7
    } else {
        0
    }
}

#[trusted]
#[ensures(*value == 9)]
#[refine_spec(where Marker: ConditionalModel, [ensures(result == old(*value))])]
pub fn conditional_replace<Marker>(value: &mut usize, _marker: &Marker) -> usize {
    let before = *value;
    *value = 9;
    before
}

#[trusted]
#[refine_spec(where Value: ConditionalModel, [ensures(result === old(value))])]
pub fn conditional_identity<Value>(value: Value) -> Value {
    value
}

/// A test-only observation, with no connection to `Iterator::next` by itself.
pub trait IteratorObservation: Iterator {
    #[pure]
    fn remaining(&self) -> Seq<Self::Item>;
}

pub use prusti_contracts::iter_model::ModeledIterator;

#[trusted]
#[refine_spec(where I: ModeledIterator, [
    ensures(match result {
        Some(_) => old(iterator.remaining()).len() > Int::from(0),
        None => old(iterator.remaining()).len() == Int::from(0),
    })
])]
pub fn modeled_advance<I: Iterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}

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

pub trait ReadRef<'a> {
    type Item;
    fn get(&self) -> &'a Self::Item;
}
pub struct Borrowed<'a, T> {
    pub value: &'a T,
}
#[refine_trait_spec]
impl<'a, T> ReadRef<'a> for Borrowed<'a, T> {
    type Item = T;
    #[ensures(result === self.value)]
    fn get(&self) -> &'a T {
        self.value
    }
}
pub trait ReadSize {
    fn size(&self) -> usize;
}
pub struct BorrowedArray<'a, const N: usize> {
    pub value: &'a [u8; N],
}
#[refine_trait_spec]
impl<'a, const N: usize> ReadSize for BorrowedArray<'a, N> {
    #[ensures(result == N)]
    fn size(&self) -> usize {
        N
    }
}

pub trait Reborrow {
    fn reborrow<'b>(&'b self) -> &'b u8;
}
#[refine_trait_spec]
impl<'a> Reborrow for Borrowed<'a, u8> {
    #[ensures(*result == *self.value)]
    fn reborrow<'b>(&'b self) -> &'b u8 {
        self.value
    }
}

pub trait RefinedFinite {
    type Item;
    #[pure]
    fn remaining(&self) -> Seq<Self::Item>;
    #[ensures(match &result {
        Some(value) => old(self.remaining()).len() > Int::from(0)
            && *value === *old(self.remaining())[0]
            && self.remaining() == old(self.remaining())[1..],
        None => old(self.remaining()).len() == Int::from(0)
            && self.remaining() == old(self.remaining()),
    })]
    fn advance(&mut self) -> Option<Self::Item>;
}
pub struct RefinedOnce {
    pub value: u32,
    pub available: bool,
}
#[refine_trait_spec]
impl RefinedFinite for RefinedOnce {
    type Item = u32;
    #[pure]
    fn remaining(&self) -> Seq<u32> {
        if self.available {
            Seq::single(self.value)
        } else {
            Seq::new()
        }
    }
    #[ensures(match &result {
        Some(value) => old(self.remaining()).len() > Int::from(0)
            && *value === *old(self.remaining())[0]
            && self.remaining() == old(self.remaining())[1..],
        None => old(self.remaining()).len() == Int::from(0)
            && self.remaining() == old(self.remaining()),
    })]
    fn advance(&mut self) -> Option<u32> {
        if self.available {
            self.available = false;
            Some(self.value)
        } else {
            None
        }
    }
}
#[ensures(match &result {
    Some(value) => old(it.remaining()).len() > Int::from(0)
        && *value === *old(it.remaining())[0]
        && it.remaining() == old(it.remaining())[1..],
    None => old(it.remaining()).len() == Int::from(0)
        && it.remaining() == old(it.remaining()),
})]
pub fn refined_advance<I: RefinedFinite>(it: &mut I) -> Option<I::Item> {
    it.advance()
}
