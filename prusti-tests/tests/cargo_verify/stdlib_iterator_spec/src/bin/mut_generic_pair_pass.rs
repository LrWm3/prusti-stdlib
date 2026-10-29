use prusti_contracts::*;
#[ensures(result === old(value))]
fn identity<T>(value: T) -> T {
    value
}
fn visit(first: &mut u32, second: &mut u32) {
    let (first, second) = identity((first, second));
    *first = 7;
    *second = 8;
    prusti_assert!(*first == 7);
    prusti_assert!(*second == 8);
}
#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut first, &mut second);
    assert_eq!((first, second), (7, 8));
}
