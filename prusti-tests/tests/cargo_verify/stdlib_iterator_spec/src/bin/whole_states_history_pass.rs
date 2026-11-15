use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(forall(|j: Int| (Int::from(0) <= j && j < Int::from(mut_slice_len(&values)) && j != Int::from(index)) ==>
    *sequence_resource_states(&mut_slice_contents(&values))[j] == old(*sequence_resource_states(&mut_slice_contents(&values))[j])))]
fn update(values: &mut [&mut u32], index: usize) {
    *values[index] = 7;
}

#[trusted]
fn main() {
    let (mut first, mut second) = (2, 1);
    update(&mut [&mut first, &mut second], 1);
    assert_eq!((first, second), (2, 7));
}
