use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    let references = ghost! { mut_iter_refs(&iterator) };
    let _ = iterator.next();
    if let Some(second) = iterator.next() {
        prusti_assert!(mut_ref_same_address(&second, &*(*references)[0])); // expected-error: assertion might not hold
    }
}
fn main() {}
