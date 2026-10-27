use prusti_contracts::*;

#[requires(start < values.len())]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(start, &replacement))]
fn replace(values: &mut [usize], start: usize, replacement: usize) {
    match values.get_mut(start..) {
        Some(part) => part[0] = replacement,
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {
    let mut values = [1usize, 2, 3];
    replace(&mut values, 1, 7);
    assert!(values[0] == 1 && values[1] == 7 && values[2] == 3);
}
