use prusti_contracts::*;

#[requires(index < mut_slice_len(&values))]
#[requires(other < mut_slice_len(&values))]
#[requires(index != other)]
#[requires(*mut_slice_referent_value(&values, Int::from(other)) == 1)]
fn update(values: &mut [&mut u32], index: usize, other: usize) {
    *values[index] = 7;
    prusti_assert!(*mut_slice_referent_value(&values, Int::from(other)) == 7); // expected-error: assertion might not hold
    prusti_assert!(*mut_slice_referent_value(&values, Int::from(index)) == 7);
}

#[trusted]
fn main() {
    let mut first = 1;
    let mut second = 2;
    update(&mut [&mut first, &mut second], 1, 0);
    assert_eq!(first, 1);
    assert_eq!(second, 7);
}
