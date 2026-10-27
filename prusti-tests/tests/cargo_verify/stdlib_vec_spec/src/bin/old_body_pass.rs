use prusti_contracts::*;

#[requires(n < usize::MAX)]
fn bump(mut n: usize) {
    n += 1;
    let created_after_entry = 17;
    prusti_assert!(n == old(n) + 1 && created_after_entry == 17);
}

#[ensures(result == n)]
fn count(n: usize) -> usize {
    let mut i = 0;
    loop {
        body_invariant!(n == old(n));
        body_invariant!(i <= n);
        if i == n {
            break;
        }
        i += 1;
    }
    i
}

fn main() {
    bump(4);
    assert!(count(0) == 0);
    assert!(count(8) == 8);
}
