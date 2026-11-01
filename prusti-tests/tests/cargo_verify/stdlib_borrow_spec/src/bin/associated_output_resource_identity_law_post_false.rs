use prusti_contracts::*;

#[ensures(result.len() == old(values.len()))]
#[ensures(false)] // expected-error: postcondition might not hold
fn identity<T>(values: &mut [T]) -> &mut [T] {
    values
}

#[trusted]
fn main() {
    let mut item = 7usize;
    let mut values = [&mut item];
    let selected = identity(&mut values);
    *selected[0] = 11;
    assert!(item == 11);
}
