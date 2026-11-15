use prusti_contracts::*;
fn read(values: &mut [&mut u32]) {
    let _value = ghost! { mut_slice_referent_value(&values, Int::from(mut_slice_len(&values))) }; // expected-error: the referent observation index may be out of bounds
}
#[trusted]
fn main() {}
