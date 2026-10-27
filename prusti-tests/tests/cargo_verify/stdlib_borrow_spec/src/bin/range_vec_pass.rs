use prusti_contracts::*;

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

fn main() {
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
