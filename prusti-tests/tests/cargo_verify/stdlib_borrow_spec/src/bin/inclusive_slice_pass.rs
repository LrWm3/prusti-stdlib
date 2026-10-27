use prusti_contracts::*;

#[requires(start <= end && end < values.len())]
#[ensures(values[start] == replacement)]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
#[ensures(Seq::from_slice(&*values)[Int::from(end) + Int::from(1)..]
    == old(Seq::from_slice(&*values))[Int::from(end) + Int::from(1)..])]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(start, &replacement))]
fn inclusive(values: &mut [usize], start: usize, end: usize, replacement: usize) {
    let part = &mut values[start..=end];
    part[0] = replacement;
}

#[requires(end < values.len())]
#[ensures(values[0] == replacement)]
#[ensures(Seq::from_slice(&*values)[Int::from(end) + Int::from(1)..]
    == old(Seq::from_slice(&*values))[Int::from(end) + Int::from(1)..])]
fn to_inclusive(values: &mut [usize], end: usize, replacement: usize) {
    let part = &mut values[..=end];
    part[0] = replacement;
}

fn main() {
    let mut values = [1usize, 2, 3, 4];
    inclusive(&mut values, 1, 2, 7);
    to_inclusive(&mut values, 0, 9);
    assert!(values[0] == 9 && values[1] == 7 && values[3] == 4);
}
