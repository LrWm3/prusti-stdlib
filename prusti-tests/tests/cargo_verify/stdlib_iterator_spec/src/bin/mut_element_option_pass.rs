use prusti_contracts::*;

#[requires(index < values.len())]
fn visit(values: &mut [Option<&mut u32>], index: usize) {
    if let Some(value) = &mut values[index] {
        **value = 7;
        let updated = **value;
        prusti_assert!(updated == 7);
    }
    if let Some(value) = &mut values[index] {
        let restored = **value;
        prusti_assert!(restored == 7);
    }
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut [Some(&mut value)], 0);
    assert_eq!(value, 7);
    visit(&mut [None], 0);
}
