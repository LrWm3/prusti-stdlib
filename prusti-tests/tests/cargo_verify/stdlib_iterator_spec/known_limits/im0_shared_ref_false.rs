use prusti_contracts::*;

#[requires(values.len() == 2 && *values[0] == 1 && *values[1] == 2)]
fn wrong(values: &[&u32]) {
    prusti_assert!(false);
}

#[trusted]
fn main() {}
