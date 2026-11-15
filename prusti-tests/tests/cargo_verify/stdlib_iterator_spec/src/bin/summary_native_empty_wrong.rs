use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;
#[requires(values.len() == 0)]
fn visit<T>(values: &mut [T]) {
    let mut it = values.iter_mut();
    match it.next() {
        None => {},
        Some(_) => { prusti_assert!(false); },
    }
    match it.next() {
        None => { prusti_assert!(false); }, // expected-error: assertion might not hold
        Some(_) => {},
    }
}
fn main() {}
