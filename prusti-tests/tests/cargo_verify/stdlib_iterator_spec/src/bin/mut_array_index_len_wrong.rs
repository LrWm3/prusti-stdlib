use prusti_contracts::*;

fn main() {
    let mut values = [3u32, 7u32];
    let slice = &mut values[..];
    prusti_assert!(slice.len() == 3); // expected-error: assertion might not hold
}
