use prusti_contracts::*;

#[requires(index < values.len())]
#[ensures(values[index] == replacement)]
#[ensures(values.len() == old(values.len()))]
fn replace_selected(values: &mut [usize], index: usize, replacement: usize) {
    let original = values[index];
    match values.get_mut(index) {
        Some(selected) => {
            assert!(*selected == original);
            let reborrowed = &mut *selected;
            *reborrowed = replacement;
        }
        None => {
            prusti_assert!(false);
        }
    }
    assert!(values[index] == replacement);
}

#[requires(index >= values.len())]
#[ensures(Seq::from_slice(values) == old(Seq::from_slice(&*values)))]
fn missing(values: &mut [usize], index: usize) {
    let before = ghost! { Seq::from_slice(&*values) };
    match values.get_mut(index) {
        Some(_) => {
            prusti_assert!(false);
        }
        None => {
            prusti_assert!(Seq::from_slice(values) == *before);
        }
    }
}

fn main() {
    let mut values = [1usize, 2, 3];
    replace_selected(&mut values, 1, 7);
    assert!(values[1] == 7);
    missing(&mut values, 3);
}
