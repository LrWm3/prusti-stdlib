use prusti_contracts::*;

#[ensures(result == N)]
fn length<T, const N: usize>(values: &mut [T; N]) -> usize {
    let slice = &mut values[..];
    slice.len()
}

fn main() {
    let mut values = [3u32, 7u32];
    let count = length(&mut values);
    prusti_assert!(count == 2);
}
