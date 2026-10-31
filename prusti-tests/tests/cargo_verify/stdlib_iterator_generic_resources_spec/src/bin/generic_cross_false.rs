use prusti_contracts::*;

fn visit<T>(values: &mut [T]) {
    let original = ghost! { Seq::from_slice(&*values) };
    let initial_states = ghost! { sequence_resource_states(&*original) };
    iterator_generic_resource_specs::preserve_all(values);
    prusti_assert!(false); // expected-error: assertion might not hold
    prusti_assert!(Seq::from_slice(&*values) == *original);
    prusti_assert!(sequence_resource_states(&Seq::from_slice(&*values)) == *initial_states);
}

struct Owned(u32);
#[trusted]
fn main() {
    visit::<&mut u32>(&mut []);
    let (mut a, mut b) = (1, 2);
    visit(&mut [&mut a, &mut b]);
    assert_eq!((a, b), (1, 2));
    let mut owned = [Owned(3), Owned(4)];
    visit(&mut owned);
    assert_eq!((owned[0].0, owned[1].0), (3, 4));
    visit(&mut [&a, &b]);
    visit(&mut [(), ()]);
}
