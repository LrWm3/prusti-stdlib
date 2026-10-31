use prusti_contracts::*;

#[requires(limit <= values.len())]
#[requires(index < values.len())]
fn check(values: &mut [&mut u32], limit: usize, index: usize) {
    let before = *values[index];
    let original = ghost! { Seq::from_slice(&*values) };
    iterator_prefix_resource_specs::set_prefix(values, limit);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    let actual = *values[index];
    if index < limit {
        prusti_assert!(actual == 7);
    } else {
        prusti_assert!(actual == before);
    }
}

#[trusted]
fn main() {
    iterator_prefix_resource_specs::set_prefix(&mut [], 0);
    let (mut a, mut b, mut c) = (1, 2, 3);
    check(&mut [&mut a, &mut b, &mut c], 0, 2);
    assert_eq!((a, b, c), (1, 2, 3));
    check(&mut [&mut a, &mut b, &mut c], 2, 1);
    assert_eq!((a, b, c), (7, 7, 3));
    check(&mut [&mut a, &mut b, &mut c], 2, 2);
    assert_eq!((a, b, c), (7, 7, 3));
    check(&mut [&mut a, &mut b, &mut c], 3, 2);
    assert_eq!((a, b, c), (7, 7, 7));
}
