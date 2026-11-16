use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;

#[requires(limit <= values.len())]
#[ensures(values.len() == old(values.len()))]
#[ensures(forall(|i: usize| i < limit ==> values[i] == 7))]
#[ensures(forall(|i: usize| limit <= i && i < values.len() ==> values[i] == old(values[i])))]
fn visit(values: &mut [u32], limit: usize) {
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *completed);
        body_invariant!(position <= limit && limit <= len);
        body_invariant!(limit == old(limit));
        body_invariant!((*original).len() == Int::from(len));
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        if position == limit { break; }
        match it.next() {
            Some(value) => {
                *value = 7;
                completed = ghost! { (*completed).append(Seq::single_ref(&*value)) };
                position += 1;
            }
            None => break,
        }
    }
    prusti_assert!(position == limit);
    drop(it);
    prusti_assert!(Seq::from_slice(&*values) == (*completed).append((*original)[limit..]));
    prusti_assert!(forall(|i: usize| i < limit ==> values[i] == 7));
    prusti_assert!(forall(|i: usize| limit <= i && i < values.len() ==> values[i] == *(*original)[i]));
}

#[trusted]
fn main() {
    visit(&mut [], 0);
    let mut values = [1, 2, 3];
    visit(&mut values, 2);
    assert_eq!(values, [7, 7, 3]);
}
