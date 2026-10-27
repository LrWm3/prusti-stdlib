use prusti_contracts::*;

#[requires(start < end && end <= values.len())]
#[ensures(values[start] == replacement)]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
#[ensures(Seq::from_slice(&*values)[end..] == old(Seq::from_slice(&*values))[end..])]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(start, &replacement))]
fn checked(values: &mut [usize], start: usize, end: usize, replacement: usize) {
    match values.get_mut(start..end) {
        Some(part) => {
            prusti_assert!(part.len() == end - start);
            part[0] = replacement;
        }
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {
    let mut values = [1usize, 2, 3];
    checked(&mut values, 1, 3, 8);
    assert!(values[1] == 8 && values[0] == 1 && values[2] == 3);
}
