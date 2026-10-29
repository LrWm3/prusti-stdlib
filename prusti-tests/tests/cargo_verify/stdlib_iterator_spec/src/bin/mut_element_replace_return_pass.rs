use prusti_contracts::*;
#[requires(index < values.len())]
fn visit<'a>(values: &mut [&'a mut u32], index: usize, replacement: &'a mut u32) {
    *replacement = 9;
    values[index] = replacement;
}
#[trusted]
fn main() {
    let (mut first, mut second, mut replacement) = (1, 2, 3);
    {
        let mut values = [&mut first, &mut second];
        visit(&mut values, 0, &mut replacement);
        assert_eq!(*values[0], 9);
        assert_eq!(*values[1], 2);
    }
    assert_eq!((first, second, replacement), (1, 2, 9));
}
