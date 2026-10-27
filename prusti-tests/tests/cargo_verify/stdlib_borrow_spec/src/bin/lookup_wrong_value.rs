use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3];
    match values.get_mut(1) {
        Some(selected) => *selected = 7,
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(values[1] == 2); // expected-error: assertion might not hold
}
