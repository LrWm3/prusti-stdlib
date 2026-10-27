use borrow_specs::replace_lookup;

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    values.push(Owned { value: 10 });
    values.push(Owned { value: 20 });
    replace_lookup(&mut values, 1, Owned { value: 30 });
    assert!(values[0].value == 10 && values[1].value == 30);
}
