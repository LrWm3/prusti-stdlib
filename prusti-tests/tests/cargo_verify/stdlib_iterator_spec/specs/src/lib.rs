#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::{iter_model::ModeledIterator, *};

#[pure]
#[ensures(result == prusti_contracts::alloc_spec::vec::vec_contents(value))]
pub fn view<T>(value: &Vec<T>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[ensures(match &result {
    Some(value) => old(iterator.remaining()).len() > Int::from(0)
        && *value === *old(iterator.remaining())[0]
        && iterator.remaining() == old(iterator.remaining())[1..],
    None => old(iterator.remaining()).len() == Int::from(0)
        && iterator.remaining() == old(iterator.remaining()),
})]
pub fn advance<I: ModeledIterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}

#[ensures(Int::from(result) == iterator.remaining().len())]
pub fn remaining_len<I: ModeledIterator + ExactSizeIterator>(iterator: &I) -> usize {
    iterator.len()
}

#[ensures((result.0 == 0 && result.1 == 0)
    || Int::from(result.1) + Int::from(1) == Int::from(result.0))]
pub fn advance_with_lengths<I: ModeledIterator + ExactSizeIterator>(
    iterator: &mut I,
) -> (usize, usize) {
    // Method syntax on `&mut I` can select its forwarding implementation,
    // which does not itself carry the sealed model capability.
    let before = ExactSizeIterator::len(&*iterator);
    let result = iterator.next();
    let after = ExactSizeIterator::len(&*iterator);
    match result {
        Some(_) => {
            prusti_assert!(after + 1 == before);
        }
        None => {
            prusti_assert!(before == 0);
            prusti_assert!(after == 0);
        }
    }
    (before, after)
}

#[requires(expected == values.len())]
#[requires(stop_after > 0)]
#[ensures(result <= expected)]
#[ensures(result <= stop_after)]
#[ensures(result == if expected < stop_after { expected } else { stop_after })]
pub fn prefix_owned<T>(values: Vec<T>, expected: usize, stop_after: usize) -> usize {
    let mut seen = 0usize;
    prusti_for!(iterator, for _value in values => {
        body_invariant!(expected == old(expected));
        body_invariant!(stop_after == old(stop_after));
        body_invariant!(seen < stop_after);
        body_invariant!(Int::from(seen) + iterator.remaining().len() + Int::from(1)
            == Int::from(expected));
        seen += 1;
        if seen == stop_after {
            break;
        }
    });
    seen
}

#[requires(values.len() > 0)]
#[ensures(result === &values[0])]
pub fn first_shared<'a, T>(values: &'a [T]) -> &'a T {
    let mut iterator = values.iter();
    advance(&mut iterator).unwrap()
}

#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_shared<T>(values: &[T], expected: usize) -> usize {
    let mut iterator = values.iter();
    let before = ghost! { iterator.remaining() };
    let source = ghost! { values };
    let mut count = 0usize;
    loop {
        body_invariant!(expected == old(expected));
        body_invariant!(count <= expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[count..]);
        body_invariant!(forall(|i: usize| i < expected ==> *(*before)[i] === &(*source)[i]));
        match advance(&mut iterator) {
            Some(value) => {
                prusti_assert!(value === &(*source)[count]);
                count += 1;
            }
            None => {
                prusti_assert!(count == expected);
                break;
            }
        }
    }
    count
}

// The explicit length keeps the pre-move value available as a loop invariant.
#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_owned<T>(values: Vec<T>, expected: usize) -> usize {
    let before = ghost! { view(&values) };
    let mut iterator = values.into_iter();
    let mut count = 0usize;
    loop {
        body_invariant!(expected == old(expected));
        body_invariant!(count <= expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[count..]);
        match iterator.next() {
            Some(_) => count += 1,
            None => {
                prusti_assert!(count == expected);
                break;
            }
        }
    }
    count
}

#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_for<T>(values: Vec<T>, expected: usize) -> usize {
    let mut count = 0usize;
    prusti_for!(iterator, for _value in values => {
        body_invariant!(expected == old(expected));
        // The body starts after `next` has removed the current element.
        body_invariant!(Int::from(count) + iterator.remaining().len() + Int::from(1) == Int::from(expected));
        count += 1;
    });
    count
}

#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_native_for<T>(values: Vec<T>, expected: usize) -> usize {
    let mut count = 0usize;
    for _value in values {
        body_invariant!(expected == old(expected));
        body_invariant!(Int::from(count)
            + for_iterator::<alloc::vec::IntoIter<T>>().remaining().len()
            + Int::from(1) == Int::from(expected));
        count += 1;
    }
    count
}

#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_shared_for<T>(values: &[T], expected: usize) -> usize {
    let mut count = 0usize;
    prusti_for!(iterator, for _value in values => {
        body_invariant!(expected == old(expected));
        body_invariant!(Int::from(count)
            + iterator.remaining().len()
            + Int::from(1) == Int::from(expected));
        count += 1;
    });
    count
}

#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_shared_for_source<T>(values: &[T], expected: usize) -> usize {
    let mut count = 0usize;
    prusti_for!(iterator, for value in values => init {
        let before = ghost! { iterator.remaining() };
    } {
        body_invariant!(expected == old(expected));
        body_invariant!(count < expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[count + 1..]);
        body_invariant!(forall(|i: usize| i < expected ==> *(*before)[i] === &values[i]));
        body_invariant!(value === &values[count]);
        prusti_assert!(value === &values[count]);
        count += 1;
    });
    count
}

/// Test-only snapshot of the unyielded elements of a mutable slice iterator.
/// This does not describe elements already yielded or restore the source slice.
#[pure]
#[trusted]
pub fn mut_remaining<T>(_iter: &core::slice::IterMut<'_, T>) -> Seq<T> {
    Seq::new()
}

// Reviewed against the pinned implementation: construction starts at the
// beginning, and each forward step yields the first remaining element.
#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[ensures(mut_remaining(&result) == old(Seq::from_slice(&*self)))]
    fn iter_mut(&mut self) -> core::slice::IterMut<'_, T>;
}

#[extern_spec]
impl<'a, T> Iterator for core::slice::IterMut<'a, T> {
    #[trusted]
    #[ensures(match &result {
        Some(value) => old(mut_remaining(&*self)).len() > Int::from(0)
            && Ghost::new_ref(&**value) == Ghost::new_ref(&*old(mut_remaining(&*self))[0])
            && mut_remaining(&*self) == old(mut_remaining(&*self))[1..],
        None => old(mut_remaining(&*self)).len() == Int::from(0)
            && mut_remaining(&*self) == old(mut_remaining(&*self)),
    })]
    fn next(&mut self) -> Option<&'a mut T>;
}
