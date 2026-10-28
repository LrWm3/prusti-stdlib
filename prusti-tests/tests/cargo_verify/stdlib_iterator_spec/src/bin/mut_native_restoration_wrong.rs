use prusti_contracts::*;

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut it = values.iter_mut();
    let first = it.next();
    let second = it.next();
    match (first, second) {
        (Some(first), Some(second)) => { *first = 7; *second = 8; }
        _ => { prusti_assert!(false); }
    }
    match it.next() {
        None => (),
        Some(_) => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[0] == 99); // expected-error: assertion might not hold
    prusti_assert!(values[1] == 8);
}

fn main() {}
