use prusti_contracts::*;

#[requires(index < values.len())]
fn replace<'a>(values: &mut [Option<&'a mut u32>], index: usize, replacement: &'a mut u32) {
    let previous = core::mem::replace(&mut values[index], Some(replacement));
    if let Some(old) = previous {
        *old = 7;
        if let Some(current) = &mut values[index] {
            **current = 11;
            let updated = **current;
            prusti_assert!(updated == 11);
        }
        let retained = *old;
        prusti_assert!(retained == 7);
    }
    if let Some(current) = &mut values[index] {
        **current = 13;
        let updated = **current;
        prusti_assert!(updated == 14); // expected-error: assertion might not hold
    }
}

#[trusted]
fn main() {
    let mut old = 1;
    let mut replacement = 2;
    replace(&mut [Some(&mut old)], 0, &mut replacement);
    assert_eq!(old, 7);
    assert_eq!(replacement, 13);
    let mut empty_replacement = 3;
    replace(&mut [None], 0, &mut empty_replacement);
    assert_eq!(empty_replacement, 13);
}
