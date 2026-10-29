use prusti_contracts::*;
#[requires(index < values.len())]
#[requires(other < values.len())]
#[requires(index != other)]
fn visit<'a>(values: &mut [&'a mut u32], index: usize, other: usize, replacement: &'a mut u32) {
    let before = *values[other];
    *replacement = 9;
    values[index] = replacement;
    let actual = *values[index];
    let framed = *values[other];
    prusti_assert!(actual == 8); // expected-error: assertion might not hold
    prusti_assert!(framed == before);
}
#[trusted]
fn main() {
    let (mut first, mut second, mut replacement) = (1, 2, 3);
    {
        let mut values = [&mut first, &mut second];
        visit(&mut values, 0, 1, &mut replacement);
        assert_eq!(*values[0], 9);
        assert_eq!(*values[1], 2);
    }
    assert_eq!((first, second, replacement), (1, 2, 9));
}
