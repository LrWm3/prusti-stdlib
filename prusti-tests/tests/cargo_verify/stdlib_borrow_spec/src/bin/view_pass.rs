use borrow_specs::view;
use prusti_contracts::*;

struct Owned {
    value: usize,
}

#[requires(first < values.len())]
#[requires(second < values.len())]
#[ensures(view(values) == old(view(&*values)))]
fn restore<Element>(values: &mut Vec<Element>, first: usize, second: usize) {
    let before = ghost! { view(&*values) };
    let length = values.len();
    borrow_specs::swap_view(values, first, second);
    assert!(values.len() == length);
    prusti_assert!(values[first] === *before[second]);
    prusti_assert!(values[second] === *before[first]);
    borrow_specs::swap_view(values, first, second);
    prusti_assert!(view(values) == *before);
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 3 });
    values.push(Owned { value: 7 });
    restore(&mut values, 0, 1);
    assert!(values[0].value == 3);
    assert!(values[1].value == 7);
}
