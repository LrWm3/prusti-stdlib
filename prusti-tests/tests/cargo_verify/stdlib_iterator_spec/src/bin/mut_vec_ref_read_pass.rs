use prusti_contracts::*;

#[requires(index < vector.len())]
fn read(vector: &mut Vec<&mut u32>, index: usize) -> u32 {
    *vector[index]
}

#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    let mut vector = vec![&mut a, &mut b];
    assert_eq!(read(&mut vector, 1), 2);
}
