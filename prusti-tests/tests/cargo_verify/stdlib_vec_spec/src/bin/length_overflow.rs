use prusti_contracts::*;

// Zero-sized elements can attain this length without allocating a buffer.
#[requires(values.len() == usize::MAX)]
fn wrong(values: &mut Vec<()>) {
    values.push(()); // expected-error: precondition might not hold
}

fn main() {}
