#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

#[pure]
#[ensures(result == prusti_contracts::alloc_spec::vec::vec_contents(value))]
pub fn view<T>(value: &Vec<T>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[requires(value.len() < usize::MAX)]
#[ensures(view(value) == old(view(&*value)).append(Seq::single_ref(&item)))]
pub fn append_one<T>(value: &mut Vec<T>, item: T) {
    value.push(item);
}

#[requires(value.len() < usize::MAX)]
#[ensures(result === item)]
#[ensures(view(value) == old(view(&*value)))]
pub fn round_trip<T>(value: &mut Vec<T>, item: T) -> T {
    value.push(item);
    value.pop().unwrap()
}

/// Export the Vec view and referent-state contract across a generic boundary.
#[ensures(Seq::from_slice(&*result) == old(view(&*value)))]
#[after_expiry(value => view(value) == before_expiry(Seq::from_slice(&*result)))]
#[ensures({
    let source = old(view(&*value));
    let result_values = Seq::from_slice(&*result);
    forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&result_values, j) == old(sequence_resource_state(&source, j)),
        triggers = [(result_values[j],)])
})]
#[after_expiry(value[_] => {
    let source = old(view(&*value));
    forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j) == before_expiry(sequence_resource_state(&source, j)),
        triggers = [(source[j],)])
})]
pub fn mutable_view<T>(value: &mut Vec<T>) -> &mut [T] {
    value.as_mut_slice()
}

// A definition-site opaque element group becomes a concrete nested lifetime
// here. Both return stages still have to preserve the caller's permissions.
pub fn concrete_reference_view<'a, 'b>(
    value: &'a mut Vec<&'b mut u32>,
) -> &'a mut [&'b mut u32] {
    value.as_mut_slice()
}
