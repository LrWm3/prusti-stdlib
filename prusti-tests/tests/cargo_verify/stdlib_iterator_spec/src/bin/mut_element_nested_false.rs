use prusti_contracts::*;
#[requires(index < values.len())]
#[requires(other < values.len())]
#[requires(index != other)]
fn visit(values: &mut [&mut &mut u32], index: usize, other: usize) {
    let before = **values[other];
    **values[index] = 7;
    let updated = **values[index];
    let framed = **values[other];
    prusti_assert!(updated == 7);
    prusti_assert!(false); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    let (mut first_ref, mut second_ref) = (&mut first, &mut second);
    visit(&mut [&mut first_ref, &mut second_ref], 0, 1);
    assert_eq!((first, second), (7, 2));
}
