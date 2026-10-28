use prusti_contracts::*;

#[requires(values.len() == 1 && *values[0] == 1)]
fn visit(values: &mut [&u32]) {
    let mut it = values.iter_mut();
    match it.next() {
        Some(value) => { prusti_assert!(**value == 1); }
        None => { prusti_assert!(false); }
    }
}

#[trusted]
fn main() { visit(&mut [&1]); }
