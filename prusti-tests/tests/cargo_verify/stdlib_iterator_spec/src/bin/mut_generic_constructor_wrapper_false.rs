use prusti_contracts::*;
struct Wrapper<T> {
    value: T,
}

#[ensures(result === old(value))]
#[ensures(resource_state(&result) == old(resource_state(&value)))]
#[after_expiry(value => resource_state(&value) == before_expiry(resource_state(&result)))]
fn relay<T>(value: Wrapper<T>) -> Wrapper<T> {
    value
}

fn main() {
    let mut number = 1;
    let wrapped = relay(Wrapper { value: &mut number });
    prusti_assert!(*wrapped.value == 1);
    *wrapped.value = 7;
    prusti_assert!(false); // expected-error: assertion might not hold
}
