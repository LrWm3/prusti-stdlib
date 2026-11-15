use prusti_contracts::*;
#[requires(index < N)]
#[ensures(Ghost::new_ref(&result) == old(Ghost::new_ref(&values[index])))]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values))
    .update(index, &*old(Ghost::new_ref(&replacement))))]
fn replace<T, const N: usize>(values: &mut [T; N], index: usize, replacement: T) -> T {
    core::mem::replace(&mut values[index], replacement)
}
struct Owned { value: u32 }
fn main() {
    let mut values = [Owned { value: 4 }, Owned { value: 11 }, Owned { value: 15 }];
    let previous = replace(&mut values, 1, Owned { value: 7 });
    prusti_assert!(previous.value == 7); // expected-error: assertion might not hold
    prusti_assert!(values[0].value == 4);
    prusti_assert!(values[1].value == 7);
    prusti_assert!(values[2].value == 15);
}
