use prusti_contracts::*;

#[ensures(result.len() == old(values.len()))]
#[ensures(Seq::from_slice(result) == old(Seq::from_slice(values)))]
fn identity<T>(values: &mut [T]) -> &mut [T] {
    values
}

fn main() {
    let mut item = 7usize;
    let mut values = [&mut item];
    let selected = identity(&mut values);
    *selected[0] = 11;
    assert!(item == 11);
}
