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

#[requires(start < end && end <= values.len())]
#[ensures(values[start] == replacement)]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
#[ensures(Seq::from_slice(&*values)[end..] == old(Seq::from_slice(&*values))[end..])]
fn slice_range(values: &mut [usize], start: usize, end: usize, replacement: usize) {
    let part = &mut values[start..end];
    part[0] = replacement;
}

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(start < end && end <= values.len())]
#[ensures(values[start] == replacement)]
#[ensures(view(&*values)[..start] == old(view(&*values))[..start])]
#[ensures(view(&*values)[end..] == old(view(&*values))[end..])]
fn vec_range(values: &mut Vec<usize>, start: usize, end: usize, replacement: usize) {
    let part = &mut values[start..end];
    part[0] = replacement;
}

#[requires(values.len() >= 2)]
#[ensures(values[1] == 7)]
fn slice(values: &mut [usize]) {
    let part = &mut values[0..2];
    prusti_assert!(part.len() == 2);
    part[1] = 7;
    prusti_assert!(values[1] == 7);
}

fn main() {
    let mut values = [1usize, 2, 3];
    slice(&mut values);
    assert!(values[1] == 7);
    checked(&mut values, 1, 3, 8);
    assert!(values[1] == 8);
    slice_range(&mut values, 1, 3, 9);
    let mut vec = Vec::new();
    vec.push(1usize);
    vec.push(2);
    vec.push(3);
    let part = &mut vec[0..2];
    prusti_assert!(part.len() == 2);
    part[1] = 7;
    prusti_assert!(vec[1] == 7);
    prusti_assert!(vec[2] == 3);
    vec_range(&mut vec, 1, 3, 10);
    assert!(vec[1] == 10 && vec[0] == 1);
}
