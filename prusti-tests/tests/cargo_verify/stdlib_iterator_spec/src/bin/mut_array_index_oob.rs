use prusti_contracts::*;

fn main() {
    let mut values = [3u32, 7u32];
    let _slice = &mut values[..3]; // expected-error: precondition might not hold
}
