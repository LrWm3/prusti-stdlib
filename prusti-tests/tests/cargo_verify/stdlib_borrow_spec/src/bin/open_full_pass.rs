use prusti_contracts::*;

#[requires(index < values.len())]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(index, &replacement))]
fn replace(values: &mut [usize], index: usize, replacement: usize) {
    let all = &mut values[..];
    all[index] = replacement;
}

fn main() {
    let mut values = [1usize, 2, 3];
    replace(&mut values, 1, 7);
    assert!(values[0] == 1 && values[1] == 7 && values[2] == 3);
}
