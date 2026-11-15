use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[requires(*mut_slice_referent_value(&values, Int::from(index)) == 1)]
fn update(values: &mut [&mut u32], index: usize) {
    let elements = ghost! { mut_slice_contents(&values) };
    let initial = ghost! { sequence_resource_states(&*elements) };
    *values[index] = 7;
    let current = ghost! { sequence_resource_states(&*elements) };
    prusti_assert!((*current).len() == (*initial).len());
    prusti_assert!(*(*current)[Int::from(index)] != *(*initial)[Int::from(index)]);
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < (*current).len() && j != Int::from(index)) ==>
        *(*current)[j] == *(*initial)[j]));
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < (*current).len()) ==>
        *(*current)[j] == sequence_resource_state(&*elements, j)));
}

#[trusted]
fn main() {
    let (mut first, mut second) = (2, 1);
    update(&mut [&mut first, &mut second], 1);
    assert_eq!((first, second), (2, 7));
}
