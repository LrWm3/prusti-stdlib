use prusti_contracts::*;

#[ensures(resource_state(&*result) == old(resource_state(&*value)))]
fn direct<T>(value: &mut T) -> &mut T { value }

#[ensures(match &result { // expected-error: postcondition might not hold
    Some(returned) => resource_state(&**returned) != old(resource_state(&*value)),
    None => false,
})]
fn wrapped<T>(value: &mut T) -> Option<&mut T> { Some(value) }

#[trusted]
fn main() {
    let mut value = 1u32;
    let mut reference = &mut value;
    **direct(&mut reference) = 7;
    match wrapped(&mut reference) { Some(returned) => **returned = 8, None => unreachable!() }
    assert_eq!(value, 8);
}
