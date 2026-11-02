use prusti_contracts::*;

#[ensures(result == match old(&*slot) {
    Some(slice) => mut_slice_len(slice),
    None => 0,
})]
#[ensures(false)] // expected-error: postcondition might not hold
fn length<T>(slot: &mut Option<&mut [T]>) -> usize {
    match slot {
        Some(slice) => slice.len(),
        None => 0,
    }
}

fn visit<T>(values: &mut [T]) {
    let expected = values.len();
    let mut slot = Some(values);
    let actual = length(&mut slot);
    prusti_assert!(actual == expected);
}

struct Token { value: usize }

fn main() {
    let mut values = [Token { value: 4 }, Token { value: 9 }];
    visit(&mut values);
    let mut empty: [Token; 0] = [];
    visit(&mut empty);
    let mut absent: Option<&mut [Token]> = None;
    let actual = length(&mut absent);
    prusti_assert!(actual == 0);
}
