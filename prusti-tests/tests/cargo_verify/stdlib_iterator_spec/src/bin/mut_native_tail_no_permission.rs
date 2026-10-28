use prusti_contracts::*;

#[requires(mut_slice_len(&*mut_iter_tail(&*it)) == 2)]
fn wrong(it: &mut core::slice::IterMut<'_, u32>) {
    let first = it.next();
    prusti_assert!(mut_slice_contents(&*old(mut_iter_tail(&*it))).len() == Int::from(2)); // expected-error: insufficient permission to read mutable slice contents
    if let Some(value) = first { *value = 7; }
}
fn main() {}
