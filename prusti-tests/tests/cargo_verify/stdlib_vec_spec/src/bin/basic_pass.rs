use prusti_contracts::*;
use vec_specs::view;

struct Owned {
    value: usize,
}

fn main() {
    let mut values = Vec::new();
    assert!(values.is_empty());
    assert!(values.len() == 0);
    prusti_assert!(view(&values) == Seq::new());
    values.push(Owned { value: 7 });
    assert!(values.len() == 1);
    assert!(!values.is_empty());
    assert!(values[0].value == 7);
    assert!(values.as_slice()[0].value == 7);
    let previous = values.pop().unwrap();
    assert!(previous.value == 7);
    assert!(values.is_empty());
    assert!(matches!(values.pop(), None));
}
