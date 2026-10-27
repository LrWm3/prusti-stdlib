use prusti_contracts::*;

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(start < end && end <= values.len())]
#[ensures(view(&*values) == old(view(&*values)).update(start, &replacement))]
fn replace(values: &mut Vec<usize>, start: usize, end: usize, replacement: usize) {
    match values.get_mut(start..end) {
        Some(part) => part[0] = replacement,
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    values.push(3);
    replace(&mut values, 1, 3, 7);
    assert!(values[0] == 1 && values[1] == 7 && values[2] == 3);
}
