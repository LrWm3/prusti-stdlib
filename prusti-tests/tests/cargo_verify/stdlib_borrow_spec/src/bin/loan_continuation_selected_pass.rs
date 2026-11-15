use prusti_contracts::*;
#[requires(index < N)]
#[ensures(Ghost::new_ref(&*result) == old(Ghost::new_ref(&values[index])))]
#[after_expiry(Seq::from_slice(&*values) == old(Seq::from_slice(&*values))
    .update(index, &*before_expiry(Ghost::new_ref(&*result))))]
fn selected<T, const N: usize>(values: &mut [T; N], index: usize) -> &mut T {
    &mut values[index]
}
struct Owned { value: u32 }
fn main() {
    let mut values = [Owned { value: 4 }, Owned { value: 11 }, Owned { value: 15 }];
    let returned = selected(&mut values, 1);
    returned.value = 7;
    prusti_assert!(values[1].value == 7);
    prusti_assert!(values[0].value == 4 && values[2].value == 15);
}
