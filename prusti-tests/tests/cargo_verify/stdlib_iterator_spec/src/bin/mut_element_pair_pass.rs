use prusti_contracts::*;

struct Pair<'a, 'b> { first: &'a mut u32, second: &'b mut u32 }

#[requires(index < values.len())]
fn visit(values: &mut [Pair<'_, '_>], index: usize) {
    let before = *values[index].second;
    *values[index].first = 7;
    let updated = *values[index].first;
    let framed = *values[index].second;
    prusti_assert!(updated == 7);
    prusti_assert!(framed == before);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [Pair { first: &mut first, second: &mut second }], 0);
    assert_eq!((first, second), (7, 2));
}
