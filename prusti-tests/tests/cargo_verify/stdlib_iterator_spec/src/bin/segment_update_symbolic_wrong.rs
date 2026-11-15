use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(*mut_slice_referent_value(&values, Int::from(index)) == 8)] // expected-error: postcondition might not hold
#[ensures(forall(|j: Int| (Int::from(0) <= j && j < Int::from(mut_slice_len(&values)) && j != Int::from(index)) ==>
    *mut_slice_referent_value(&values, j) == old(*mut_slice_referent_value(&values, j))))]
fn update(values: &mut [&mut u32], index: usize) {
    *values[index] = 7;
}

#[trusted]
fn main() {
    let mut first = 1;
    let mut second = 2;
    update(&mut [&mut first, &mut second], 1);
    assert_eq!(first, 1);
    assert_eq!(second, 7);
}
