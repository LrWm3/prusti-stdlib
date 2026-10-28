use prusti_contracts::*;

fn visit(n: usize) {
    let scalar = ghost! { 42u32 };
    let sequence = ghost! { seq![1u32, 2u32] };
    let mut i = 0usize;
    loop {
        body_invariant!(i <= n);
        body_invariant!(*scalar == *scalar);
        body_invariant!((*sequence).len() == (*sequence).len());
        if i == n { break; }
        i += 1;
    }
    prusti_assert!(*scalar == 42);
    prusti_assert!((*sequence).len() == Int::from(2));
    prusti_assert!(*(*sequence)[0] == 2); // expected-error: assertion might not hold
}

fn main() {}
