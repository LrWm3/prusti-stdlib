use prusti_contracts::*;
fn sequence(n: usize) {
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    loop {
        body_invariant!(position <= n);
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(forall(|i: Int| Int::from(0) <= i && i < Int::from(position) ==> *(*completed)[i] == 7)); // expected-error: loop invariant might not be preserved
        if position == n { break; }
        let value = 8u32;
        completed = ghost! { (*completed).append(Seq::single_ref(&value)) };
        position += 1;
    }
    prusti_assert!(forall(|i: usize| i < n ==> *(*completed)[i] == 7));
}
fn main() {}
