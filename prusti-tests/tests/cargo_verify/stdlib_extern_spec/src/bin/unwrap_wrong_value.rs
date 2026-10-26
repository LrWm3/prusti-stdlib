extern crate stdlib_specs;
use prusti_contracts::*;

struct Token {
    value: u32,
}

fn main() {
    let token = Some(Token { value: 7 }).unwrap();
    prusti_assert!(token.value == 8); // expected-error: assertion might not hold
}
