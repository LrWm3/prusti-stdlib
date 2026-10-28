use prusti_contracts::*;

#[pure]
fn tail<'a, T>(it: &core::slice::IterMut<'a, T>) -> Ghost<&'a mut [T]> {
    mut_iter_tail(it) // expected-unsupported: cannot encode method body `tail`: `prusti_contracts` builtin Spec(MutIterTail) cannot be used in executable code
}
fn main() {}
