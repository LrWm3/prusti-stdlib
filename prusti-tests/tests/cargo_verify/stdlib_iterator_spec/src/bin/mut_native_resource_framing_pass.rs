use prusti_contracts::*;

#[requires(values.len() == 2)]
fn visit(values: &mut [&mut u32]) {
    let first_before = *values[0];
    let second_before = *values[1];
    let mut it = values.iter_mut();
    match it.next() {
        Some(first) => {
            let observed = **first;
            prusti_assert!(observed == first_before);
            **first = 7;
        },
        None => { prusti_assert!(false); },
    }
    match it.next() {
        Some(second) => {
            let observed = **second;
            prusti_assert!(observed == second_before);
        },
        None => { prusti_assert!(false); },
    }
    match it.next() {
        Some(_) => { prusti_assert!(false); },
        None => (),
    }
    drop(it);
    let first = *values[0];
    let second = *values[1];
    prusti_assert!(first == 7);
    prusti_assert!(second == second_before);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second]);
    assert_eq!((first, second), (7, 2));
}
