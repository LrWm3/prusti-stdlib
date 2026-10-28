use prusti_contracts::*;

#[requires(values.len() == 2)]
#[ensures(values[0] == 7 && values[1] == 7)]
fn visit(values: &mut [u32]) {
    let mut it = values.iter_mut();
    match it.next() {
        Some(value) => *value = 7,
        None => { prusti_assert!(false); }
    }
    match it.next() {
        Some(value) => *value = 7,
        None => { prusti_assert!(false); }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [1, 2]); }
