use prusti_contracts::*;

#[requires(values.len() == 2 && *values[0] == 1 && *values[1] == 1)]
fn visit(values: &mut [&u32]) {
    let mut it = values.iter_mut();
    match (it.next(), it.next()) {
        (Some(first), Some(second)) => {
            prusti_assert!(**first == 1);
            prusti_assert!(**second == 1);
        }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(*values[0] == 1);
    prusti_assert!(*values[1] == 2); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let shared = 1;
    visit(&mut [&shared, &shared]);
    let other = 1;
    visit(&mut [&shared, &other]);
}
