extern crate stdlib_specs;
use prusti_contracts::*;

struct Token {
    value: u32,
}

fn main() {
    let mut slot = Token { value: 7 };
    let _previous = std::mem::replace(&mut slot, Token { value: 11 });
    prusti_assert!(slot.value == 7); // expected-error: assertion might not hold
}
