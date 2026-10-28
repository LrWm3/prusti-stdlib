use prusti_contracts::*;

#[requires(values.len() == 2)]
#[after_expiry(values[1] == before_expiry(*result))] // expected-error: pledge postcondition might not hold
fn select(values: &mut [u32]) -> &mut u32 {
    let mut index = 0;
    let selected = &mut values[index];
    index = 1;
    let _ = index;
    selected
}

fn main() {}
