use prusti_contracts::*;

#[requires(0 < end && end <= values.len())]
#[ensures(values[0] == replacement)]
#[ensures(Seq::from_slice(&*values)[end..] == old(Seq::from_slice(&*values))[end..])]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(0, &replacement))]
fn replace(values: &mut [usize], end: usize, replacement: usize) {
    let part = &mut values[..end];
    part[0] = replacement;
}

#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)))]
fn empty(values: &mut [usize]) {
    match values.get_mut(..0) {
        Some(part) => {
            prusti_assert!(part.len() == 0);
        }
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {
    let mut values = [1usize, 2, 3];
    replace(&mut values, 2, 9);
    empty(&mut values);
    assert!(values[0] == 9 && values[1] == 2 && values[2] == 3);
}
