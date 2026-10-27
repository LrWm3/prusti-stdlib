use prusti_contracts::*;

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(index < values.len())]
#[ensures(view(&*values) == old(view(&*values)).update(index, &replacement))]
fn replace(values: &mut Vec<usize>, index: usize, replacement: usize) {
    match values.get_mut(index) {
        Some(value) => *value = replacement,
        None => {
            prusti_assert!(false);
        }
    }
}

fn main() {}
