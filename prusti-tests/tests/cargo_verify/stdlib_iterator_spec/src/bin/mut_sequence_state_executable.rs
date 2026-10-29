use prusti_contracts::*;
fn observe(values: Seq<u32>) {
    let state = sequence_resource_state(&values, Int::from(0)); // expected-unsupported: cannot encode method body `observe`: `prusti_contracts` builtin SequenceResourceState cannot be used in executable code
}
fn main() {}
