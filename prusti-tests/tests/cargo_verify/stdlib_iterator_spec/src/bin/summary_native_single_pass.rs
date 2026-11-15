use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;
#[requires(values.len() == 1 && values[0] == 1)]
#[ensures(values[0] == 7)]
fn visit(values: &mut [u32]) {
    let mut it = values.iter_mut();
    match it.next() {
        Some(value) => *value = 7,
        None => { prusti_assert!(false); },
    }
    drop(it);
}
fn main() {}
