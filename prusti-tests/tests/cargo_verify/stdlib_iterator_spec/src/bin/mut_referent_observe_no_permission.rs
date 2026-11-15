use prusti_contracts::*;
fn read(values: &&mut [&mut u32]) {
    let _value = ghost! { mut_slice_referent_value(values, Int::from(0)) }; // expected-error: insufficient permission to read mutable slice contents
}
#[trusted]
fn main() {}
