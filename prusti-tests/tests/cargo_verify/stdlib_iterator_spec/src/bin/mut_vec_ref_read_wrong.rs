use prusti_contracts::*;

#[requires(index < vector.len())]
fn read(vector: &mut Vec<&mut u32>, index: usize) -> u32 {
    let actual = *vector[index];
    prusti_assert!(actual == 7); // expected-error: assertion might not hold
    actual
}

#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    let mut vector = vec![&mut a, &mut b];
    assert_eq!(read(&mut vector, 1), 2);
}
