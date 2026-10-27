use prusti_contracts::*;

#[requires(end >= values.len())]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)))]
fn missing(values: &mut [usize], end: usize) {
    match values.get_mut(..=end) {
        Some(_) => {
            prusti_assert!(false);
        }
        None => {}
    }
}

#[requires(values.len() >= 4)]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)))]
fn empty(values: &mut [usize]) {
    match values.get_mut(4..=3) {
        Some(part) => {
            prusti_assert!(part.len() == 0);
        }
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {
    let mut values = [1usize, 2, 3, 4];
    missing(&mut values, usize::MAX);
    empty(&mut values);
    assert!(values[0] == 1 && values[2] == 3);
}
