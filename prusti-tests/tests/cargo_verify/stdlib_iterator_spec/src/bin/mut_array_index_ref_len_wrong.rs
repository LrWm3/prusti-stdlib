use prusti_contracts::*;

fn main() {
    let mut left = 3u32;
    let mut right = 7u32;
    let mut values = [&mut left, &mut right];
    let slice = &mut values[..];
    prusti_assert!(slice.len() == 3); // expected-error: assertion might not hold
}
