use prusti_contracts::*;

fn discard<T>(value: T) {
    drop(value);
}

struct Pair<'a, 'b> {
    first: &'a mut u32,
    second: &'b mut u32,
}

#[ensures(*first == 7 && *second == 8)]
fn visit(first: &mut u32, second: &mut u32) {
    discard(Pair { first: &mut *first, second: &mut *second });
    *first = 7;
    *second = 8;
    prusti_assert!(false); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut first, &mut second);
    assert_eq!((first, second), (7, 8));
}
