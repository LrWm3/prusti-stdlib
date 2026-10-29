// IM4: symbolic-length array expiry framing is not yet accepted.
use prusti_contracts::*;
#[ensures(result === old(value))]
#[ensures(resource_state(&result) == old(resource_state(&value)))]
#[after_expiry(resource_state(&value) == before_expiry(resource_state(&result)))]
fn identity<T>(value: T) -> T { value }
#[ensures(result === old(values))]
#[ensures(resource_state(&result) == old(resource_state(&values)))]
#[after_expiry(resource_state(&values) == before_expiry(resource_state(&result)))]
fn relay<const N: usize>(values: [&mut u32; N]) -> [&mut u32; N] {
    identity(values)
}
#[trusted]
fn main() {
    let mut value = 1;
    let values = relay([&mut value]);
    *values[0] = 7;
    assert_eq!(value, 7);
}
