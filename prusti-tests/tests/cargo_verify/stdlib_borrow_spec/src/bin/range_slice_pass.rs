use prusti_contracts::*;

#[requires(start < end && end <= values.len())]
#[ensures(values[start] == replacement)]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
#[ensures(Seq::from_slice(&*values)[end..] == old(Seq::from_slice(&*values))[end..])]
fn slice_range(values: &mut [usize], start: usize, end: usize, replacement: usize) {
    let part = &mut values[start..end];
    part[0] = replacement;
}

#[requires(values.len() >= 2)]
#[ensures(values[1] == 7)]
fn slice(values: &mut [usize]) {
    let part = &mut values[0..2];
    prusti_assert!(part.len() == 2);
    part[1] = 7;
    prusti_assert!(values[1] == 7);
}

fn main() {
    let mut values = [1usize, 2, 3];
    slice(&mut values);
    assert!(values[1] == 7);
    slice_range(&mut values, 1, 3, 9);
    assert!(values[1] == 9);
}
