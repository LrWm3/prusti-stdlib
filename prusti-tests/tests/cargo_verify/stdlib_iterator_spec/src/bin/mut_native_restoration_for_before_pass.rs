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
    prusti_assert!(position == len);
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

    let mut evaluations = 0;
    let mut initializations = 0;
    let mut advances = 0;
    prusti_for!(iterator, 'outer: for value in {
        evaluations += 1;
        [1, 2, 3, 4]
    } => init {
        initializations += 1;
        assert_eq!(iterator.len(), 4);
    } before_next {
        advances += 1;
    } {
        if value == 2 { continue 'outer; }
        if value == 4 { break 'outer; }
    });
    assert_eq!((evaluations, initializations, advances), (1, 1, 4));

    let mut empty_advances = 0;
    prusti_for!(iterator, for _value in [0u8; 0] => before_next {
        empty_advances += 1;
    } {});
    assert_eq!(empty_advances, 1);
}
