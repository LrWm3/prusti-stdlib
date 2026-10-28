use prusti_contracts::*;

// Substituting a lifetime-free T removes the generalized RegionsIn(T)
// projection before the mutable reference. The current predicate encoder
// cannot reuse the signature's numeric index in the substituted tuple.
#[trusted]
fn borrow<'a, T>(input: (T, &'a mut u32)) -> &'a mut u32 { input.1 }

fn main() {
    let mut value = 1;
    let borrowed = borrow(((), &mut value));
    *borrowed = 2;
    value = 3; // expected-unsupported: cannot encode method body `main`: function-call resource projection a0↓1 changes under substitution
    assert!(value == 3);
}
