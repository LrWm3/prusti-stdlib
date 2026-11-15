use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;
#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut it = values.iter_mut();
    let first = match it.next() {
        Some(value) => value,
        None => { prusti_assert!(false); return; }
    };
    let second = match it.next() {
        Some(value) => value,
        None => { prusti_assert!(false); return; }
    };
    *first = 7;
    *second = 8;
    drop(it);
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 8);
}
fn main() {}
