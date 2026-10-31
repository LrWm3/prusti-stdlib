use prusti_contracts::*;

struct Owned(u32);

#[requires(index < values.len())]
fn check(values: &mut [&mut Owned], index: usize) {
    let before = values[index].0;
    let original = ghost! { Seq::from_slice(&*values) };
    iterator_generic_resource_specs::preserve_all(values);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    let actual = values[index].0;
    prusti_assert!(actual != before); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut a, mut b) = (Owned(1), Owned(2));
    check(&mut [&mut a, &mut b], 1);
    assert_eq!((a.0, b.0), (1, 2));
}
