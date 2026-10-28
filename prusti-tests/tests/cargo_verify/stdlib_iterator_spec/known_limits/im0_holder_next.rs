use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32 }

#[requires(values.len() == 1)]
fn visit(values: &mut [Holder<'_>]) {
    let mut it = values.iter_mut();
    let first = it.next();
    drop(it);
    match first {
        Some(value) => *value.value = 7,
        None => { prusti_assert!(false); }
    }
    prusti_assert!(*values[0].value == 7);
}

#[trusted]
fn main() {
    let mut first = 1;
    visit(&mut [Holder { value: &mut first }]);
    assert_eq!(first, 7);
}
