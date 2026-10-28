use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;

#[requires(values.is_empty())]
fn empty<T>(values: &mut [T]) {
    let mut it = values.iter_mut();
    match it.next() {
        None => (),
        Some(_) => { prusti_assert!(false); }
    }
    match it.next() {
        None => (),
        Some(_) => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values.is_empty());
}

#[trusted]
fn main() { empty::<u32>(&mut []); }
