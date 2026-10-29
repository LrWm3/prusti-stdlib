use prusti_contracts::*;

struct Pair<'a> {
    first: &'a mut u32,
    second: &'a mut u32,
}

#[requires(index < values.len())]
fn visit(values: &mut [Pair<'_>], index: usize) {
    let before = *values[index].second;
    *values[index].first = 7;
    let updated = *values[index].first;
    let framed = *values[index].second;
    prusti_assert!(updated == 7);
    prusti_assert!(framed == 7); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(
        &mut [Pair {
            first: &mut first,
            second: &mut second,
        }],
        0,
    );
    assert_eq!((first, second), (7, 2));
}
