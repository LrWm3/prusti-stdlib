use prusti_contracts::*;

#[pure]
fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(start <= end && end < values.len())]
#[ensures(Ghost::new_ref(&values[start]) == old(Ghost::new_ref(&replacement)))]
#[ensures(view(&*values)[..start] == old(view(&*values))[..start])]
#[ensures(view(&*values)[Int::from(end) + Int::from(1)..]
    == old(view(&*values))[Int::from(end) + Int::from(1)..])]
fn replace<Element>(values: &mut Vec<Element>, start: usize, end: usize, replacement: Element) {
    let part = &mut values[start..=end];
    part[0] = replacement;
}

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 10 });
    values.push(Owned { value: 20 });
    values.push(Owned { value: 30 });
    replace(&mut values, 1, 1, Owned { value: 40 });
    assert!(values[0].value == 10 && values[1].value == 40 && values[2].value == 30);
}
