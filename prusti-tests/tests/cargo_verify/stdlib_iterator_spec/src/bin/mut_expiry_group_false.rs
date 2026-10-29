use prusti_contracts::*;

#[ensures(result === old(value))]
#[ensures(resource_state(&result) == old(resource_state(&value)))]
#[after_expiry(value[_] => resource_state(&value) == before_expiry(resource_state(&result)))]
fn relay<T>(value: T) -> T {
    value
}

fn main() {
    let mut value = 1;
    let returned = relay(&mut value);
    prusti_assert!(*returned == 1);
    *returned = 7;
    prusti_assert!(value == 7);
    prusti_assert!(false); // expected-error: assertion might not hold
}
