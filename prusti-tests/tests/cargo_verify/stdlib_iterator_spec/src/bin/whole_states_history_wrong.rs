use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(*sequence_resource_states(&mut_slice_contents(&values))[Int::from(index)] == old(*sequence_resource_states(&mut_slice_contents(&values))[Int::from(index)]))] // expected-error: postcondition might not hold
fn update(values: &mut [&mut u32], index: usize) {
    *values[index] = 7;
}
fn main() {}
