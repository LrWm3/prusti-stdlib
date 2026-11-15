use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut it = values.iter_mut();
    loop {
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) <= len);
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
    prusti_assert!(false); // expected-error: assertion might not hold
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

fn main() {}
