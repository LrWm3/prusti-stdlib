use prusti_contracts::*;

#[requires(index < vector.len())]
fn update(vector: &mut Vec<&mut u32>, index: usize) {
    let values = vector.as_mut_slice();
    *values[index] = 7;
    let actual = *vector[index];
    prusti_assert!(actual == 8); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    update(&mut vec![&mut a, &mut b], 1);
    assert_eq!((a, b), (1, 7));
}
