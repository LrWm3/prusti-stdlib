use prusti_contracts::*;

fn visit(values: &mut [&mut u32]) {
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [&mut u32] };
    let mut prefix = ghost! { Seq::<&mut u32>::new() };
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
                **value = 7;
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

#[trusted]
fn main() {
    visit(&mut []);
    let (mut a, mut b) = (1, 2);
    visit(&mut [&mut a, &mut b]);
    assert_eq!((a, b), (7, 7));
}
