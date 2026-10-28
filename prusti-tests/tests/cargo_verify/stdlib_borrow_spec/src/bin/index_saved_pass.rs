use prusti_contracts::*;

#[requires(values.len() == 2)]
#[after_expiry(values[0] == before_expiry(*result))]
fn select(values: &mut [u32]) -> &mut u32 {
    let mut index = 0;
    let selected = &mut values[index];
    index = 1;
    let _ = index;
    selected
}

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let selected = select(values);
    *selected = 7;
    prusti_assert!(values[0] == 7);
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 2]);
}
fn main() { runtime_check(); }
