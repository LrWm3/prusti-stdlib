use prusti_contracts::*;

#[requires(index < values.len())]
#[requires(other < values.len() && index != other)]
fn visit(values: &mut [&mut u32], index: usize, other: usize) {
    let untouched = *values[other];
    *values[index] = 7;
    let updated = *values[index];
    let framed = *values[other];
    prusti_assert!(updated == 7);
    prusti_assert!(framed == untouched);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second], 0, 1);
    assert_eq!((first, second), (7, 2));
}
