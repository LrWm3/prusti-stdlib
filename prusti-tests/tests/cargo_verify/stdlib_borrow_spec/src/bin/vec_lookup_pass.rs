use prusti_contracts::*;

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(index < values.len())]
#[ensures(view(&*values) == old(view(&*values)).update(index, &replacement))]
fn replace(values: &mut Vec<usize>, index: usize, replacement: usize) {
    match values.get_mut(index) {
        Some(value) => {
            let reborrow = &mut *value;
            *reborrow = replacement;
        }
        None => {
            prusti_assert!(false);
        }
    }
}

#[requires(index >= values.len())]
#[ensures(view(&*values) == old(view(&*values)))]
fn missing(values: &mut Vec<usize>, index: usize) {
    match values.get_mut(index) {
        Some(_) => {
            prusti_assert!(false);
        }
        None => {}
    }
}

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    values.push(3);
    replace(&mut values, 1, 7);
    missing(&mut values, 3);
    assert!(values[0] == 1 && values[1] == 7 && values[2] == 3);
}
