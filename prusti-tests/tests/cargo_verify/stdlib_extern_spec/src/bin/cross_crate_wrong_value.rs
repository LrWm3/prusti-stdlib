use prusti_contracts::*;

struct Token {
    value: u32,
}

fn main() {
    let mut slot = stdlib_specs::unwrap_owned(Token { value: 13 });
    let previous = stdlib_specs::replace_owned(&mut slot, Token { value: 29 });
    prusti_assert!(previous.value == 29); // expected-error: assertion might not hold
}
