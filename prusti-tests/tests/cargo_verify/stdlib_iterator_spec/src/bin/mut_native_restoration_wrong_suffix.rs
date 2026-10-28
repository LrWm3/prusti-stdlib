use prusti_contracts::*;

#[requires(values.len() >= 2 && stop == 1 && values[1] == 3)]
fn visit(values: &mut [u32], stop: usize) {
    let len = values.len();
    let initial = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *completed);
        body_invariant!(*initial == *initial);
        body_invariant!(position <= len);
        body_invariant!(position <= stop);
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7));
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*initial)[position..]);
        if position == stop { break; }
        match it.next() {
            Some(value) => {
                *value = 7;
                completed = ghost! { (*completed).append(Seq::single(7u32)) };
                position += 1;
            }
            None => break,
        }
    }
    drop(it);
    prusti_assert!(position == len || position == stop);
    prusti_assert!(forall(|i: usize| i < position ==> values[i] == 7));
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
}

fn main() {}
