// A nonempty partial traversal must not make the final path inconsistent.
use prusti_contracts::*;

#[requires(0 < limit)]
#[requires(limit < mut_slice_len(&values))]
#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(forall(|j: Int| (Int::from(0) <= j && j < Int::from(limit)) ==>
    *mut_slice_referent_value(&values, j) == 7))]
#[ensures(forall(|j: Int| (Int::from(limit) <= j && j < Int::from(mut_slice_len(&values))) ==>
    *mut_slice_referent_value(&values, j) == old(*mut_slice_referent_value(&values, j))))]
fn visit(values: &mut [&mut u32], limit: usize) {
    let mut exemplar = 7u32;
    let exemplar_ref = &mut exemplar;
    let expected = ghost! { resource_state(&exemplar_ref) };
    drop(exemplar_ref);
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let initial_states = ghost! { sequence_resource_states(&*original) };
    let owner = ghost! { (&*values) as *const [&mut u32] };
    let mut prefix = ghost! { Seq::<&mut u32>::new() };
    let mut states = ghost! { Seq::<ResourceState>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix, *states);
        body_invariant!(*original == *original);
        body_invariant!((*original).len() == Int::from(len));
        body_invariant!((*initial_states).len() == Int::from(len));
        body_invariant!((*prefix).len() == Int::from(position));
        body_invariant!(position <= limit);
        body_invariant!(limit <= len);
        body_invariant!(limit == old(limit));
        body_invariant!((*states).len() == Int::from(position));
        body_invariant!(forall(|j: Int| Int::from(0) <= j && j < Int::from(position) ==> *(*states)[j] == *expected));
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        body_invariant!(forall(|j: Int| (Int::from(position) <= j && j < Int::from(len)) ==>
            sequence_resource_state(&mut_slice_contents(&*mut_iter_tail(&it)), j - Int::from(position)) == *(*initial_states)[j]));
        if position == limit { break; }
        prusti_assert!(position < limit);
        let next_position = position + 1;
        match it.next() {
            Some(value) => {
                **value = 7;
                states = ghost! { (*states).append(Seq::single(resource_state(&*value))) };
                prefix = ghost! { (*prefix).append(Seq::single_ref(&*value)) };
                position = next_position;
            }
            None => break,
        }
    }
    prusti_assert!(position == limit);
    prusti_assert!(forall(|j: Int| (Int::from(limit) <= j && j < Int::from(len)) ==>
        sequence_resource_state(&mut_slice_contents(&*mut_iter_tail(&it)), j - Int::from(limit)) == *(*initial_states)[j]));
    drop(it);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    prusti_assert!({
        let source = mut_slice_contents(&values);
        forall(move |j: Int| (Int::from(limit) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j) == *(*initial_states)[j],
            triggers = [(source[j],), ((*initial_states)[j],)])
    });
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < Int::from(limit)) ==>
        *mut_slice_referent_value(&values, j) == 7));
    prusti_assert!(forall(|j: Int| (Int::from(limit) <= j && j < Int::from(len)) ==>
        *mut_slice_referent_value(&values, j) == old(*mut_slice_referent_value(&values, j))));

    prusti_assert!(false); // expected-error: assertion might not hold

}

#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    visit(&mut [&mut a, &mut b], 1);
}
