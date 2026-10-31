use prusti_contracts::*;

struct Pair<'a, 'b> { first: &'a mut u32, second: &'b mut u32 }

fn visit(values: &mut [Pair<'_, '_>], index: usize) {
    let (mut first, mut second) = (7u32, 8u32);
    let exemplar_ref = Pair { first: &mut first, second: &mut second };
    let expected = ghost! { resource_state(&exemplar_ref) };
    drop(exemplar_ref);
    let len = values.len();
    let original = ghost! { Seq::from_slice(&*values) };
    let owner = ghost! { (&*values) as *const [Pair<'_, '_>] };
    let mut prefix = ghost! { Seq::<Pair<'_, '_>>::new() };
    let mut states = ghost! { Seq::<ResourceState>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix, *states);
        body_invariant!(*original == *original);
        body_invariant!(position <= len);
        body_invariant!((*states).len() == Int::from(position));
        body_invariant!(forall(|j: Int| Int::from(0) <= j && j < Int::from(position) ==> *(*states)[j] == *expected));
        body_invariant!(*prefix == (*original)[..position]);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*original)[position..]);
        match it.next() {
            Some(value) => {
                *value.first = 7;
                *value.second = 8;
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
    if index < len {
        let first = *values[index].first;
        let second = *values[index].second;
        prusti_assert!(first == 7);
        prusti_assert!(second == 8);
    }
}

#[trusted]
fn main() {
    visit(&mut [], 0);
    let (mut a, mut b, mut c, mut d) = (1, 2, 3, 4);
    visit(&mut [Pair { first: &mut a, second: &mut b }, Pair { first: &mut c, second: &mut d }], 1);
    assert_eq!((a, b, c, d), (7, 8, 7, 8));
}
