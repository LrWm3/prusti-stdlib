use prusti_contracts::*;

#[requires(!values.is_empty())]
fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_invariant!(len > 0);
        body_invariant!(position <= len);
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7));
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        match it.next() {
            Some(value) => {
                *value = 7;
                completed = ghost! { (*completed).append(Seq::single(7u32)) };
                position += 1;
            }
            None => break,
        }
    }
    prusti_assert!(position == len);
    prusti_assert!((*completed).len() == Int::from(len));
    prusti_assert!(forall(|i: Int| Int::from(0) <= i && i < Int::from(len) ==> *(*completed)[i] == 7));
}

#[trusted]
fn main() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 7]);
}
