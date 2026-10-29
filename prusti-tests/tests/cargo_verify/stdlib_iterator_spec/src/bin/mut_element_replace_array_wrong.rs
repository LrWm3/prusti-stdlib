use prusti_contracts::*;
#[requires(index < 2)]
fn visit(index: usize) {
    let (mut first, mut second, mut replacement) = (1u32, 2u32, 3u32);
    {
        let mut values = [&mut first, &mut second];
        values[index] = &mut replacement;
        *values[index] = 9;
        let actual = *values[index];
        prusti_assert!(actual == 9);
    }
    prusti_assert!(first == 1);
    prusti_assert!(second == 2);
    prusti_assert!(replacement == 8); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    visit(0);
    visit(1);
}
