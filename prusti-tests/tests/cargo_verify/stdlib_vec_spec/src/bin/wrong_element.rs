use prusti_contracts::*;

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 7 });
    prusti_assert!(values[0].value == 8); // expected-error: assertion might not hold
}
