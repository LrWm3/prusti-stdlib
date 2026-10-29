use prusti_contracts::*;

#[after_expiry(value => Ghost::new_ref(&*value) == before_expiry(Ghost::new_ref(&*result)))]
fn reborrow<T>(value: &mut T) -> &mut T {
    value
}

fn main() {
    let mut value = 1;
    *reborrow(&mut value) = 7;
    prusti_assert!(false); // expected-error: assertion might not hold
}
