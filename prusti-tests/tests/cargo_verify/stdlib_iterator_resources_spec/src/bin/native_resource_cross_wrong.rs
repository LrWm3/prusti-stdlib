use prusti_contracts::*;
fn check(values: &mut [&mut u32]) {
    let len = values.len();
    let initial = ghost! { Seq::from_slice(&*values) };
    iterator_resource_specs::set_all_referents(values);
    prusti_assert!(Seq::from_slice(&*values) == *initial);
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < Int::from(len)) ==> *mut_slice_referent_value(&values, j) == 8)); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    check(&mut []);
    let (mut a, mut b) = (1, 2);
    check(&mut [&mut a, &mut b]);
    assert_eq!((a, b), (7, 7));
}
