use prusti_contracts::*;
use iterator_resource_specs::{copy_pair_values, Pair};

fn visit(values: &mut [Pair<'_, '_>], index: usize) {
    let (mut first, mut second) = (7u32, 8u32);
    let mut exemplar = Pair { first: &mut first, second: &mut second };
    let original = ghost! { Seq::from_slice(&*values) };
    copy_pair_values(values, &mut exemplar);
    prusti_assert!(Seq::from_slice(&*values) == *original);
    if index < values.len() {
        let first = *values[index].first;
        let second = *values[index].second;
        prusti_assert!(first == 7);
        prusti_assert!(second == 7); // expected-error: assertion might not hold
    }
}

#[trusted]
fn main() {
    visit(&mut [], 0);
    let (mut a, mut b, mut c, mut d) = (1, 2, 3, 4);
    visit(&mut [Pair { first: &mut a, second: &mut b }, Pair { first: &mut c, second: &mut d }], 1);
    assert_eq!((a, b, c, d), (7, 8, 7, 8));
}
