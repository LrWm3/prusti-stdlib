use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;

#[requires(values.len() == 1 && values[0] == 1)]
fn visit(values: &mut [u32]) {
    let mut it = values.iter_mut();
    let first = it.next();
    drop(it);
    match first {
        Some(value) => { *value = 7; }
        None => { prusti_assert!(false); }
    }
    prusti_assert!(values[0] == 7);
}

#[trusted]
fn main() {
    let mut values = [1];
    visit(&mut values);
    assert_eq!(values, [7]);
}
