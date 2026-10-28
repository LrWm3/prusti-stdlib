use prusti_contracts::*;

#[requires(values.len() == 1 && values[0] == 1)]
#[ensures(forall(|i: usize| i < values.len() ==> values[i] == old(values[i]) + 6))]
fn update(values: &mut [u32]) { values[0] = 7; }

#[trusted]
fn main() {
    let mut values = [1];
    update(&mut values);
    assert_eq!(values, [7]);
}
