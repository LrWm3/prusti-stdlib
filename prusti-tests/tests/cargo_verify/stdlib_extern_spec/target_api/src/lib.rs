//! Unannotated targets for external-contract resolution probes.
use std::marker::PhantomData;

pub trait Identity {
    type Item;

    fn identity(value: Self::Item) -> Self::Item;
    fn borrow<'a>(value: &'a Self::Item) -> &'a Self::Item;
}

pub struct Owned<T>(PhantomData<T>);

pub struct ConditionalContainer<T> {
    pub value: usize,
    pub marker: PhantomData<T>,
}

impl<T> ConditionalContainer<T> {
    pub fn new() -> Self {
        Self {
            value: 0,
            marker: PhantomData,
        }
    }

    pub fn replace(&mut self, value: usize) -> usize {
        core::mem::replace(&mut self.value, value)
    }

    pub fn conditional(input: usize) -> usize {
        if input > 0 {
            7
        } else {
            0
        }
    }

    pub fn static_observe() -> usize {
        7
    }
}

impl<T> Identity for Owned<T> {
    type Item = T;

    fn identity(value: T) -> T {
        value
    }

    fn borrow<'a>(value: &'a T) -> &'a T {
        value
    }
}

pub trait Bounded {
    fn bounded(value: u32) -> u32;
}

pub trait Relay<T> {
    fn relay(value: T) -> T;
}

impl<T> Relay<T> for Owned<T> {
    fn relay(value: T) -> T {
        value
    }
}

pub struct Zero;

impl Bounded for Zero {
    fn bounded(_value: u32) -> u32 {
        0
    }
}

pub fn shared(value: &u32) -> &u32 {
    value
}

pub fn mutable(value: &mut u32) -> &mut u32 {
    value
}

pub fn copied<T: Copy>(value: T) -> T {
    value
}

pub fn static_value<T: 'static>(value: T) -> T {
    value
}

pub trait Copier<T: Copy> {
    fn copy(value: T) -> T;
}

impl<T: Copy> Copier<T> for Owned<T> {
    fn copy(value: T) -> T {
        value
    }
}

impl Bounded for [u32] {
    fn bounded(_value: u32) -> u32 {
        0
    }
}

pub fn first<'a, 'b>(first: &'a u32, _second: &'b u32) -> &'a u32 {
    first
}

pub fn static_ref(_value: &u32) -> &'static u32 {
    &37
}

pub fn outlives<'a: 'b, 'b>(first: &'a u32, _second: &'b u32) -> &'b u32 {
    first
}

pub type Borrow<'a, T> = &'a T;

pub fn aliased<'a, T>(value: Borrow<'a, T>) -> Borrow<'a, T> {
    value
}

pub fn callback(value: for<'a> fn(&'a u32) -> &'a u32) -> for<'b> fn(&'b u32) -> &'b u32 {
    value
}

pub fn static_callback(_value: fn(&'static u32) -> &'static u32) {}

pub fn bounded_identity<'a, T: 'a>(value: T, _marker: &'a ()) -> T {
    value
}

pub trait Consume {
    fn consume(self) -> u32;
}

impl Consume for Zero {
    fn consume(self) -> u32 {
        0
    }
}

pub trait Observe {
    fn observe(&self) -> u32;
}

impl Observe for Zero {
    fn observe(&self) -> u32 {
        0
    }
}

impl Observe for &u8 {
    fn observe(&self) -> u32 {
        0
    }
}
