use prusti_contracts::*;

#[requires(values.len() == 2)]
fn visit(values: &mut [&mut u32]) {
    let mut it = (&mut *values).into_iter();
    let first = it.next();
    let second = it.next();
    drop(it);
    match (first, second) {
        (Some(first), Some(second)) => {
            **first = 7;
            **second = 8;
        }
        _ => {
            prusti_assert!(false);
        }
    }
    let first = *values[0];
    let second = *values[1];
    prusti_assert!(first == 7);
    prusti_assert!(second == 8);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second]);
    assert_eq!((first, second), (7, 8));
}
