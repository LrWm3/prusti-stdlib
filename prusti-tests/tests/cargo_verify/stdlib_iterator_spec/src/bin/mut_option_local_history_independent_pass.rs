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
    let mut a = 4u32;
    let mut b = 9u32;
    let mut c = 5u32;
    let mut d = 10u32;
    let mut first_values = [&mut a, &mut b];
    let mut second_values = [&mut c, &mut d];
    let mut first: Option<&mut [&mut u32]> = Some(&mut first_values);
    let mut second: Option<&mut [&mut u32]> = Some(&mut second_values);
    let count = clear(&mut first);
    prusti_assert!(count == 2);
    let count = clear(&mut second);
    prusti_assert!(count == 2);
    prusti_assert!(matches!(first, None));
    prusti_assert!(matches!(second, None));
    drop(first);
    drop(second);
    *first_values[0] = 13;
    *second_values[0] = 17;
    let first_current = *first_values[0];
    let second_current = *second_values[0];
    prusti_assert!(first_current == 13);
    prusti_assert!(second_current == 17);
}
