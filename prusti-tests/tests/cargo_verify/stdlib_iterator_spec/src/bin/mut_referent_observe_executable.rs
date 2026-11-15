use prusti_contracts::*;
fn read(values: &mut [&mut u32]) {
    let _value = mut_slice_referent_value(&values, Int::from(0)); // expected-unsupported: cannot encode method body `read`: `prusti_contracts` builtin MutSliceReferentValue cannot be used in executable code
}
#[trusted]
fn main() {}
