use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *completed);
        body_invariant!(position <= len);
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7));
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        prusti_assert!(Seq::from_slice(unsafe { &*(*owner) }).len() == Int::from(0)); // expected-unsupported: cannot encode method body `visit`: dereference of the raw pointer `*const [u32]`
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
    prusti_assert!(forall(|i: usize| i < values.len() ==> values[i] == 7));
    prusti_assert!(position == len);
    prusti_assert!((*completed).len() == Int::from(len));
    prusti_assert!(forall(|i: Int| Int::from(0) <= i && i < Int::from(len) ==> *(*completed)[i] == 7));
}

fn main() {}
