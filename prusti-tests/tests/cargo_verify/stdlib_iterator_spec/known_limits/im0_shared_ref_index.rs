use prusti_contracts::*;

#[requires(values.len() == 2 && *values[0] == 1 && *values[1] == 2)]
fn visit(values: &[&u32]) {
    let first = values[0];
    prusti_assert!(*first == 1);
    prusti_assert!(*values[1] == 2);
}

#[trusted]
fn main() { visit(&[&1, &2]); }
