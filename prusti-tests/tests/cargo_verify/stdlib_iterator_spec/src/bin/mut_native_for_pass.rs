use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    for value in &mut *values {
        *value = 7;
    }
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

#[trusted]
fn main() {
    let mut values = [1, 2, 3];
    visit(&mut values);
    assert_eq!(values, [11, 7, 7]);
    visit(&mut []);
}
