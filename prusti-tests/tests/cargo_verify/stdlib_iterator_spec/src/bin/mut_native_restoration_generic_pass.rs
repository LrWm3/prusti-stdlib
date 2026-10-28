use prusti_contracts::*;

fn visit<T>(values: &mut [T]) {
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [T] };
    let mut prefix = ghost! { Seq::<T>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix);
        body_invariant!(*original == *original);
        body_invariant!(position <= len);
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        match it.next() {
            Some(value) => {
                prefix = ghost! { (*prefix).append(Seq::single_ref(&*value)) };
                position += 1;
            }
            None => break,
        }
    }
    drop(it);
    prusti_assert!(position == len);
    prusti_assert!(Seq::from_slice(&*values) == *original);
}

struct Owned(u32);
#[trusted]
fn main() {
    let mut values = [Owned(1), Owned(2)];
    visit(&mut values);
    assert_eq!(values[0].0, 1);
    assert_eq!(values[1].0, 2);
}
