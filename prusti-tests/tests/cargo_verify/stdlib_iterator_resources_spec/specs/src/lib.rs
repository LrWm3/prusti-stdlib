#![no_std]
use prusti_contracts::*;

#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(forall(|j: Int| (Int::from(0) <= j && j < Int::from(mut_slice_len(&values))) ==>
    *mut_slice_referent_value(&values, j) == 7))]
pub fn set_all_referents(values: &mut [&mut u32]) {
    let mut exemplar = 7u32;
    let exemplar_ref = &mut exemplar;
    let expected = ghost! { resource_state(&exemplar_ref) };
    drop(exemplar_ref);
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [&mut u32] };
    let mut prefix = ghost! { Seq::<&mut u32>::new() };
    let mut states = ghost! { Seq::<ResourceState>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix, *states);
        body_invariant!(*original == *original);
        body_invariant!(position <= len);
        body_invariant!((*states).len() == Int::from(position));
        body_invariant!(forall(|j: Int| Int::from(0) <= j && j < Int::from(position) ==> *(*states)[j] == *expected));
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        match it.next() {
            Some(value) => {
                **value = 7;
                states = ghost! { (*states).append(Seq::single(resource_state(&*value))) };
                prefix = ghost! { (*prefix).append(Seq::single_ref(&*value)) };
                position += 1;
            }
            None => break,
        }
    }
    drop(it);
    prusti_assert!(position == len);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    prusti_assert!(forall(|i: usize| i < len ==>
        *mut_slice_referent_value(&values, Int::from(i)) == 7));
}

pub struct Pair<'a, 'b> { pub first: &'a mut u32, pub second: &'b mut u32 }

#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(Ghost::new_ref(&*exemplar) == old(Ghost::new_ref(&*exemplar)))]
#[ensures(resource_state(&*exemplar) == old(resource_state(&*exemplar)))]
#[ensures({
    let source = mut_slice_contents(&values);
    let expected = old(resource_state(&*exemplar));
    forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j) == expected,
        triggers = [(source[j],)])
})]
pub fn copy_pair_values(values: &mut [Pair<'_, '_>], exemplar: &mut Pair<'_, '_>) {
    let expected = ghost! { resource_state(&*exemplar) };
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [Pair<'_, '_>] };
    let mut prefix = ghost! { Seq::<Pair<'_, '_>>::new() };
    let mut states = ghost! { Seq::<ResourceState>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix, *states);
        body_invariant!(*original == *original);
        body_invariant!(Ghost::new_ref(&*exemplar) == old(Ghost::new_ref(&*exemplar)));
        body_invariant!(resource_state(&*exemplar) == *expected);
        body_invariant!(position <= len);
        body_invariant!((*states).len() == Int::from(position));
        body_invariant!(forall(|j: Int| Int::from(0) <= j && j < Int::from(position) ==> *(*states)[j] == *expected));
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        match it.next() {
            Some(value) => {
                *value.first = *exemplar.first;
                *value.second = *exemplar.second;
                states = ghost! { (*states).append(Seq::single(resource_state(&*value))) };
                prefix = ghost! { (*prefix).append(Seq::single_ref(&*value)) };
                position += 1;
            }
            None => break,
        }
    }
    drop(it);
    prusti_assert!(position == len);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < Int::from(len)) ==>
        sequence_resource_state(&Seq::from_slice(&*values), j) == *expected));
}
