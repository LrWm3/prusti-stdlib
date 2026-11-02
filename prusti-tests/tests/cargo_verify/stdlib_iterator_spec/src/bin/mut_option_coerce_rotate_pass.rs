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
    let mut slot: Option<&mut [&mut u32]> = Some(&mut values);
    let count = clear(&mut slot);
    prusti_assert!(count == 2);
    prusti_assert!(matches!(slot, None));
    drop(slot);
    *values[0] = 13;
    let current = *values[0];
    prusti_assert!(current == 13);
    
}
