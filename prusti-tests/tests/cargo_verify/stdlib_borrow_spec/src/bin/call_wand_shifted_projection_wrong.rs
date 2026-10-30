use prusti_contracts::*;

// Substituting a lifetime-free T removes the generalized RegionsIn(T)
// projection before the mutable reference. Restoration must still select
// the mutable reference after substitution.
#[trusted]
fn borrow<'a, T>(input: (T, &'a mut u32)) -> &'a mut u32 { input.1 }

fn main() {
    let mut value = 1;
    let borrowed = borrow(((), &mut value));
    *borrowed = 2;
    value = 3;
    assert!(value == 4); // expected-error: precondition of `core::panicking::panic` (called by this macro expansion) might not hold
}
