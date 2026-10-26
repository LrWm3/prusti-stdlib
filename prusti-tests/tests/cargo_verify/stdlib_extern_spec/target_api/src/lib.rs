//! Unannotated targets for external-contract resolution probes.
use std::marker::PhantomData;

pub trait Identity {
    type Item;

    fn identity(value: Self::Item) -> Self::Item;
    fn borrow<'a>(value: &'a Self::Item) -> &'a Self::Item;
}

pub struct Owned<T>(PhantomData<T>);

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
