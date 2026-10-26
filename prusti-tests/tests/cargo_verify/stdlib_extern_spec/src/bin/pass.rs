// Force the specification crate to be loaded, even though it exports no runtime API.
extern crate stdlib_specs;
use prusti_contracts::*;

// Deliberately has no Copy, Clone, PartialEq, or Eq implementation.
struct Token {
    value: u32,
}

#[ensures(result === value)]
fn unwrap_generic<T>(value: T) -> T {
    Some(value).unwrap()
}

#[requires(matches!(value, Some(_)))]
#[ensures(old(value) === Some(result))]
fn unwrap_with_precondition<T>(value: Option<T>) -> T {
    value.unwrap()
}

#[ensures(result.0 === first)]
#[ensures(result.1 === second)]
fn replace_generic<T>(first: T, second: T) -> (T, T) {
    let mut slot = first;
    let previous = std::mem::replace(&mut slot, second);
    (previous, slot)
}

#[ensures(result === *old(&*dest))]
#[ensures(*dest === src)]
fn replace_borrowed<T>(dest: &mut T, src: T) -> T {
    std::mem::replace(dest, src)
}

#[requires(value < u8::MAX)]
#[ensures(result == value + 1)]
fn checked_increment(value: u8) -> u8 {
    value + 1
}

#[ensures(*result == old(token.value))]
#[after_expiry(token.value == before_expiry(*result))]
fn borrow_value(token: &mut Token) -> &mut u32 {
    &mut token.value
}

fn main() {
    assert!(true);
    assert!(unwrap_generic(42u32) == 42);
    let token = unwrap_generic(Token { value: 7 });
    assert!(token.value == 7);
    let token = unwrap_with_precondition(Some(token));
    assert!(token.value == 7);

    let (previous, current) = replace_generic(token, Token { value: 11 });
    assert!(previous.value == 7);
    assert!(current.value == 11);

    let mut slot = current;
    let previous = replace_borrowed(&mut slot, previous);
    assert!(previous.value == 11);
    assert!(slot.value == 7);
    assert!(checked_increment(41) == 42);

    let value = borrow_value(&mut slot);
    assert!(*value == 7);
    *value = 13;
    assert!(slot.value == 13);
}
