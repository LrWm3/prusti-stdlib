#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

#[pure]
pub fn view<Element>(values: &Vec<Element>) -> Seq<Element> {
    Seq::from_slice(values.as_slice())
}

#[requires(first < values.len())]
#[requires(second < values.len())]
#[ensures(view(values) == old(view(&*values))
    .update(first, &*old(view(&*values))[second])
    .update(second, &*old(view(&*values))[first]))]
pub fn swap_view<Element>(values: &mut Vec<Element>, first: usize, second: usize) {
    values.as_mut_slice().swap(first, second);
}

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result)
    && Ghost::new_ref(&_marker) == old(Ghost::new_ref(&_marker)))]
pub fn identity_with_marker<Marker>(value: &mut usize, _marker: Marker) -> &mut usize {
    value
}

#[ensures(match result {
    Some(value) => present && *value == old(*owner),
    None => !present,
})]
#[after_expiry(match result {
    Some(value) => *owner == before_expiry(*value),
    None => *owner == old(*owner),
})]
pub fn optional(owner: &mut usize, present: bool) -> Option<&mut usize> {
    if present {
        Some(owner)
    } else {
        None
    }
}

pub trait Borrowed {
    #[pure]
    fn observe(&self) -> usize;
    fn borrow(&mut self) -> &mut usize;
}

pub struct Cell {
    pub value: usize,
}

#[refine_trait_spec]
impl Borrowed for Cell {
    #[pure]
    fn observe(&self) -> usize {
        self.value
    }

    #[after_expiry(Ghost::new_ref(&self.value) == before_expiry(Ghost::new_ref(&*result)))]
    fn borrow(&mut self) -> &mut usize {
        &mut self.value
    }
}
