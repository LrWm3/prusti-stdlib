// This contract promises selected-value identity. Untouched deep state is
// checked separately by the no-frame negative; no frame is assumed here.
use prusti_contracts::*;

#[requires(values.len() > 0)]
#[ensures(&*result === old(&values[0]))]
fn first_element<T>(values: &mut [T]) -> &mut T {
    &mut values[0]
}

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut values = [&mut first, &mut second];
    let selected = first_element(&mut values);
    **selected = 11;
    prusti_assert!(first == 11);
}
