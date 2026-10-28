use prusti_contracts::*;

#[requires(values.len() == 2 && values[0] == 1 && values[1] == 2)]
fn visit(values: &mut [u32]) {
    let mut it = (&mut *values).into_iter();
    let first = it.next();
    drop(it);
    match first {
        Some(value) => { *value = 7; }
        None => { prusti_assert!(false); }
    }
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 2);
}

#[trusted]
fn main() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 2]);
}
