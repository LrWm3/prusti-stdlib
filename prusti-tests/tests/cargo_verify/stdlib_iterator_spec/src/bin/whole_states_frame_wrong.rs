use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[requires(other < mut_slice_len(&values))]
#[requires(index != other)]
#[requires(*mut_slice_referent_value(&values, Int::from(other)) == 1)]
fn update(values: &mut [&mut u32], index: usize, other: usize) {
    *values[index] = 7;
    let current = ghost! { sequence_resource_states(&mut_slice_contents(&values)) };
    prusti_assert!(*(*current)[Int::from(index)] == *(*current)[Int::from(other)]); // expected-error: assertion might not hold
}
fn main() {}
