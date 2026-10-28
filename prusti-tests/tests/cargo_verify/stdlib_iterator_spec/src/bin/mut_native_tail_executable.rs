use prusti_contracts::*;

fn tail<'a, T>(it: &core::slice::IterMut<'a, T>) {
    let _tail = mut_iter_tail(it); // expected-unsupported: cannot encode method body `tail`: `prusti_contracts` builtin Spec(MutIterTail) cannot be used in executable code
}
fn main() {}
