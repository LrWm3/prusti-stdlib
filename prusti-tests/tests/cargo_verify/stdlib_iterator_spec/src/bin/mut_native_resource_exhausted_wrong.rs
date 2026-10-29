use prusti_contracts::*;
#[requires(values.len() == 1)]
fn visit(values: &mut [&mut u32]) {
    let mut it = values.iter_mut();
    let first = it.next();
    match it.next() { Some(_) => { prusti_assert!(false); }, None => () }
    drop(it);
    match first { Some(value) => **value = 7, None => { prusti_assert!(false); } }
    let updated = *values[0];
    prusti_assert!(updated == 8); // expected-error: assertion might not hold
}
#[trusted]
fn main() { let mut value=1; visit(&mut [&mut value]); assert_eq!(value,7); }
