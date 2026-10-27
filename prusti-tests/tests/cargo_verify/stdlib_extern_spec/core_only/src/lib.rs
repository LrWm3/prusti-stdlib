#![no_std]

use prusti_contracts::*;

#[requires(matches!(value, Some(_)))]
#[ensures(old(value) === Some(result))]
pub fn unwrap_owned<T>(value: core::option::Option<T>) -> T {
    value.unwrap()
}

#[ensures(result === *old(&*dest))]
#[ensures(*dest === src)]
pub fn replace_owned<T>(dest: &mut T, src: T) -> T {
    core::mem::replace(dest, src)
}
