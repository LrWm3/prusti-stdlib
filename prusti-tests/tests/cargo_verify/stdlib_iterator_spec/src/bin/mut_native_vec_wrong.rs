use prusti_contracts::*;

#[requires(values.len() == 2 && values[0] == 1 && values[1] == 2)]
fn wrong(values: &mut Vec<u32>) {
    let mut it = values.iter_mut();
    let first = it.next();
    drop(it);
    match first {
        Some(value) => *value = 7,
        None => { prusti_assert!(false); }
    }
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
}

fn main() {}
