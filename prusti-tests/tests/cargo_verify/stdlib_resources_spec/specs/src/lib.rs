#![no_std]
use prusti_contracts::*;

#[ensures(result === old(value))]
#[ensures(resource_state(&result) === old(resource_state(&value)))]
#[after_expiry(resource_state(&value) === before_expiry(resource_state(&result)))]
pub fn identity<T>(value: T) -> T {
    value
}

#[ensures(result === old(value))]
#[ensures(resource_state(&result) === old(resource_state(&value)))]
#[after_expiry(resource_state(&value) === before_expiry(resource_state(&result)))]
pub fn twice<T>(value: T) -> T {
    identity(identity(value))
}

pub struct References<'a, 'b> {
    pub first: &'a mut u32,
    pub second: &'b mut u32,
}

pub struct Owned {
    pub value: u32,
}
