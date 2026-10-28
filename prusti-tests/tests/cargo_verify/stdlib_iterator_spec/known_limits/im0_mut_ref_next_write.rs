use prusti_contracts::*;

#[requires(values.len() == 2)]
fn visit(values: &mut [&mut u32]) {
    let mut it = values.iter_mut();
    let first = it.next();
    let second = it.next();
    drop(it);
    match (first, second) {
        (Some(first), Some(second)) => { **first = 7; **second = 8; }
        _ => { prusti_assert!(false); }
    }
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second]);
    assert_eq!((first, second), (7, 8));
}
