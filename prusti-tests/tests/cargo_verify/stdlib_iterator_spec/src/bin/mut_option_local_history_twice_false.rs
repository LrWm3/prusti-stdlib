use prusti_contracts::*;

#[ensures(result == match old(&*slot) {
    Some(slice) => mut_slice_len(slice),
    None => 0,
})]
#[ensures(matches!(&*slot, None))]
fn clear<T>(slot: &mut Option<&mut [T]>) -> usize {
    let count = match slot {
        Some(values) => {
            let count = values.len();
            if count == 2 { values.swap(0, 1); }
            count
        }
        None => 0,
    };
    *slot = None;
    count
}

fn main() {
    let mut left = 4u32;
    let mut right = 9u32;
    let mut values = [&mut left, &mut right];
    let mut first: Option<&mut [&mut u32]> = Some(&mut values);
    let count = clear(&mut first);
    prusti_assert!(count == 2);
    prusti_assert!(matches!(first, None));
    drop(first);
    let mut second: Option<&mut [&mut u32]> = Some(&mut values);
    let count = clear(&mut second);
    prusti_assert!(count == 2);
    prusti_assert!(matches!(second, None));
    drop(second);
    *values[0] = 17;
    let current = *values[0];
    prusti_assert!(current == 17);
    prusti_assert!(false); // expected-error: assertion might not hold
}
