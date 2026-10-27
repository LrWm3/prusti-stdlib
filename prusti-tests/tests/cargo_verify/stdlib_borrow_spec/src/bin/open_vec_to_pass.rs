use prusti_contracts::*;

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(0 < end && end <= values.len())]
#[ensures(Ghost::new_ref(&values[0]) == old(Ghost::new_ref(&replacement)))]
#[ensures(view(&*values)[end..] == old(view(&*values))[end..])]
fn replace<Element>(values: &mut Vec<Element>, end: usize, replacement: Element) {
    let part = &mut values[..end];
    part[0] = replacement;
}

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 10 });
    values.push(Owned { value: 20 });
    replace(&mut values, 1, Owned { value: 30 });
    assert!(values[0].value == 30 && values[1].value == 20);
}
