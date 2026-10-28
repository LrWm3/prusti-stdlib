use prusti_contracts::*;

#[requires(values.len() == 1)]
fn visit(values: &mut [Option<&mut u32>]) {
    let mut it = values.iter_mut();
    if let Some(Some(value)) = it.next() {
        **value = 7;
    }
    drop(it);
    if let Some(value) = &values[0] {
        prusti_assert!(**value == 7);
    }
}

#[trusted]
fn main() {
    let mut first = 1;
    visit(&mut [Some(&mut first)]);
    assert_eq!(first, 7);
    visit(&mut [None]);
}
