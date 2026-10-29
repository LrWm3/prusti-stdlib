use prusti_contracts::*;

#[after_expiry(value => Ghost::new_ref(&*value) == before_expiry(Ghost::new_ref(&*result)))]
fn reborrow<T>(value: &mut T) -> &mut T { value }

#[trusted]
fn main() {
    let mut value = 1;
    *reborrow(&mut value) = 7;
    assert_eq!(value, 7);
}
