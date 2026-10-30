use prusti_contracts::*;
fn observe(values: Seq<u32>) {
    let states = sequence_resource_states(&values); // expected-unsupported: cannot encode method body `observe`: `prusti_contracts` builtin SequenceResourceStates cannot be used in executable code
}
fn main() {}
