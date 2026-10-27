use prusti_contracts::*;

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    match values.get_mut(1) {
        Some(value) => *value = 7,
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(values[0] == 9); // expected-error: assertion might not hold
}
