use prusti_contracts::*;

#[requires(index < N)]
#[requires(other < N)]
#[requires(index != other)]
fn visit<const N: usize>(values: &mut [&mut u32; N], index: usize, other: usize) {
    let untouched = *values[other];
    *values[index] = 7;
    let updated = *values[index];
    let framed = *values[other];
    prusti_assert!(updated == 7);
    prusti_assert!(framed != untouched); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut first, mut second, mut third) = (1, 2, 3);
    visit(&mut [&mut first, &mut second, &mut third], 1, 2);
}
