use prusti_contracts::*;
use vec_specs::view;

#[requires(i < values.len())]
#[requires(values.len() < usize::MAX)]
fn append<T>(values: &mut Vec<T>, item: T, i: usize) {
    let before = ghost! { view(&*values) };
    let moved = ghost! { Ghost::new_ref(&item) };
    let n = values.len();
    vec_specs::append_one(values, item);
    assert!(values.len() == n + 1);
    prusti_assert!(view(values)[i] === before[i]);
    prusti_assert!(values[i] === *before[i]);
    prusti_assert!(values[n] === **moved);
}

#[requires(values.len() < usize::MAX)]
fn round_trip<T>(values: &mut Vec<T>, item: T) {
    let before = ghost! { view(&*values) };
    let moved = ghost! { Ghost::new_ref(&item) };
    let returned = vec_specs::round_trip(values, item);
    prusti_assert!(view(values) == *before);
    prusti_assert!(returned === **moved);
}

#[requires(values.len() > 0)]
#[requires(Int::from(i) + Int::from(1) < Int::from(values.len()))]
fn pop<T>(values: &mut Vec<T>, i: usize) {
    let before = ghost! { view(&*values) };
    let n = values.len();
    let last = values.pop().unwrap();
    assert!(values.len() == n - 1);
    prusti_assert!(last === *before[n - 1]);
    prusti_assert!(values[i] === *before[i]);
}

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 2 });
    values.push(Owned { value: 3 });
    values.push(Owned { value: 5 });
    values.push(Owned { value: 7 });
    values.push(Owned { value: 11 });
    pop(&mut values, 2);
    // These functions are themselves verified for every T and entry sequence.
    // Reestablish their length requirements after the unconstrained pop wrapper.
    let mut other = Vec::new();
    other.push(Owned { value: 13 });
    append(&mut other, Owned { value: 17 }, 0);
    let mut third = Vec::new();
    third.push(Owned { value: 19 });
    round_trip(&mut third, Owned { value: 23 });
}
