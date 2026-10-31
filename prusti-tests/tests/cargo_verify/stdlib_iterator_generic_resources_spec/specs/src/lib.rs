#![no_std]
use prusti_contracts::*;

#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(sequence_resource_states(&mut_slice_contents(&values)) ==
    old(sequence_resource_states(&mut_slice_contents(&values))))]
#[ensures({
    let source = mut_slice_contents(&values);
    forall(|j: Int| (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j) ==
            old(sequence_resource_state(&mut_slice_contents(&values), j)),
        triggers = [(source[j],)])
})]
pub fn preserve_all<T>(values: &mut [T]) {
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let initial_states = ghost! { sequence_resource_states(&*original) };
    let owner = ghost! { (&*values) as *const [T] };
    let mut prefix = ghost! { Seq::<T>::new() };
    let mut states = ghost! { Seq::<ResourceState>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix, *states);
        body_invariant!(*original == *original);
        body_invariant!(position <= len);
        body_invariant!((*states).len() == Int::from(position));
        body_invariant!(forall(|j: Int| Int::from(0) <= j && j < Int::from(position) ==> *(*states)[j] == *(*initial_states)[j]));
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        body_invariant!(forall(|j: Int| (Int::from(position) <= j && j < Int::from(len)) ==>
            sequence_resource_state(&mut_slice_contents(&*mut_iter_tail(&it)), j - Int::from(position)) == *(*initial_states)[j]));
        match it.next() {
            Some(value) => {
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
    prusti_assert!(sequence_resource_states(&Seq::from_slice(&*values)) == *initial_states);
}
