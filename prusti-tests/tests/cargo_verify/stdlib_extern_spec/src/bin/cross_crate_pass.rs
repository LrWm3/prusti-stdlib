use prusti_contracts::*;

struct Token {
    value: u32,
}

#[ensures(result.0 === first)]
#[ensures(result.1 === second)]
fn round_trip<T>(first: T, second: T) -> (T, T) {
    let mut slot = stdlib_specs::unwrap_owned(first);
    let old = stdlib_specs::replace_owned(&mut slot, second);
    (old, slot)
}

fn main() {
    let (old, new) = round_trip(Token { value: 13 }, Token { value: 29 });
    assert!(old.value == 13);
    assert!(new.value == 29);
}
