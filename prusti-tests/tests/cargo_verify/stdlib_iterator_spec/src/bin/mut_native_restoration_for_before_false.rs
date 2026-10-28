use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    prusti_for!(it, for value in &mut *values => before_next {
        body_restore_prefix!(&it, *owner, *completed);
        body_invariant!(position <= len);
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7));
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
    } {
        *value = 7;
        completed = ghost! { (*completed).append(Seq::single(7u32)) };
        position += 1;
    });
    prusti_assert!(forall(|i: usize| i < values.len() ==> values[i] == 7));
    prusti_assert!(false); // expected-error: assertion might not hold
    prusti_assert!((*completed).len() == Int::from(len));
    prusti_assert!(forall(|i: Int| Int::from(0) <= i && i < Int::from(len) ==> *(*completed)[i] == 7));
}

#[trusted]
fn main() {
    let mut empty: [u32; 0] = [];
    visit(&mut empty);
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 7]);
}
