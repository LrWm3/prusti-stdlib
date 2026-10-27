#![feature(allocator_api, slice_index_methods)]
use core::{ops::IndexMut, slice::SliceIndex};
use prusti_contracts::*;

#[extern_spec]
impl<Element, Index: SliceIndex<[Element]>> IndexMut<Index> for [Element] {
    #[trusted]
    #[requires(matches!(index.get(&*self), Some(_)))]
    #[ensures(match old(index.get(&*self)) {
        Some(previous) => Ghost::new_ref(&*result) == Ghost::new_ref(previous),
        None => false,
    })]
    #[after_expiry(self.len() == old(self.len()) && match index.get(&*self) {
        Some(current) => Ghost::new_ref(current) == before_expiry(Ghost::new_ref(&*result)),
        None => false,
    })]
    fn index_mut(&mut self, index: Index) -> &mut Index::Output;
}

#[extern_spec]
impl<Element, Index: SliceIndex<[Element]>, Allocator: core::alloc::Allocator> IndexMut<Index>
    for Vec<Element, Allocator>
{
    #[trusted]
    #[requires(matches!(index.get(self.as_slice()), Some(_)))]
    #[ensures(match old(index.get(self.as_slice())) {
        Some(previous) => Ghost::new_ref(&*result) == Ghost::new_ref(previous),
        None => false,
    })]
    #[after_expiry(self.len() == old(self.len()) && match index.get(self.as_slice()) {
        Some(current) => Ghost::new_ref(current) == before_expiry(Ghost::new_ref(&*result)),
        None => false,
    })]
    fn index_mut(&mut self, index: Index) -> &mut Index::Output;
}

#[requires(index < values.len())]
#[ensures(values[index] == replacement)]
fn assign_slice(values: &mut [usize], index: usize, replacement: usize) {
    values[index] = replacement;
}

#[requires(index < values.len())]
#[ensures(values[index] == replacement)]
fn assign_vec(values: &mut Vec<usize>, index: usize, replacement: usize) {
    values[index] = replacement;
}

fn main() {
    let mut slice = [1usize, 2, 3];
    assign_slice(&mut slice, 1, 7);
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    assign_vec(&mut values, 1, 7);
}
