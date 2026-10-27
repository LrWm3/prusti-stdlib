use borrow_specs::replace_index;
use prusti_contracts::*;

#[requires(index < values.len() && other < values.len() && index != other)]
#[ensures(values[index] == replacement && values[other] == old(values[other]))]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(index, &replacement))]
fn checked(values: &mut [usize], index: usize, other: usize, replacement: usize) {
    match values.get_mut(index) {
        Some(value) => *value = replacement,
        None => {
            prusti_assert!(false);
        }
    }
}

#[requires(index < values.len() && other < values.len() && index != other)]
#[ensures(values[index] == replacement && values[other] == old(values[other]))]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values)).update(index, &replacement))]
fn slice_index(values: &mut [usize], index: usize, other: usize, replacement: usize) {
    values[index] = replacement;
}

#[requires(index < values.len() && other < values.len() && index != other)]
#[ensures(values[index] == replacement && values[other] == old(values[other]))]
#[ensures(borrow_specs_view(&*values) == old(borrow_specs_view(&*values)).update(index, &replacement))]
fn vec_index(values: &mut Vec<usize>, index: usize, other: usize, replacement: usize) {
    values[index] = replacement;
}

#[requires(index < values.len() && other < values.len() && index != other)]
#[ensures(Ghost::new_ref(&values[other]) == old(Ghost::new_ref(&values[other])))]
#[ensures(Seq::from_slice(&*values) == old(Seq::from_slice(&*values))
    .update(index, &*old(Ghost::new_ref(&replacement))))]
fn owned<Element>(values: &mut [Element], index: usize, other: usize, replacement: Element) {
    values[index] = replacement;
}

#[pure]
fn borrow_specs_view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

struct Owned {
    value: usize,
}

fn main() {
    let mut values = [1usize, 2, 3];
    checked(&mut values, 1, 0, 4);
    slice_index(&mut values, 0, 2, 5);
    assert!(values[0] == 5 && values[1] == 4 && values[2] == 3);

    let mut vector = Vec::new();
    vector.push(1usize);
    vector.push(2);
    vector.push(3);
    vec_index(&mut vector, 2, 0, 6);
    assert!(vector[0] == 1 && vector[2] == 6);

    let mut items = [Owned { value: 10 }, Owned { value: 20 }];
    owned(&mut items, 0, 1, Owned { value: 30 });
    assert!(items[0].value == 30 && items[1].value == 20);

    let mut owned_vector = Vec::new();
    owned_vector.push(Owned { value: 10 });
    owned_vector.push(Owned { value: 20 });
    replace_index(&mut owned_vector, 0, Owned { value: 30 });
    assert!(owned_vector[0].value == 30 && owned_vector[1].value == 20);
}
