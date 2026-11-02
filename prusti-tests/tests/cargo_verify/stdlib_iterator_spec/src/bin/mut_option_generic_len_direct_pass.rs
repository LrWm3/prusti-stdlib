use prusti_contracts::*;

#[ensures(result == match old(&*slot) {
    Some(slice) => mut_slice_len(slice),
    None => 0,
})]
fn length<T>(slot: &mut Option<&mut [T]>) -> usize {
    match slot {
        Some(slice) => slice.len(),
        None => 0,
    }
}

struct Token { value: usize }

fn main() {
    let mut values = [Token { value: 4 }, Token { value: 9 }];
    {
        let mut slot: Option<&mut [Token]> = Some(&mut values);
        let actual = length(&mut slot);
        prusti_assert!(actual == 2);
    }
    let mut empty: [Token; 0] = [];
    {
        let mut slot: Option<&mut [Token]> = Some(&mut empty);
        let actual = length(&mut slot);
        prusti_assert!(actual == 0);
    }
    let mut absent: Option<&mut [Token]> = None;
    let actual = length(&mut absent);
    prusti_assert!(actual == 0);
}
