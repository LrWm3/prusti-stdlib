use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3];
    match values.get_mut(1..) {
        Some(part) => part[0] = 7,
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(values[0] == 9); // expected-error: assertion might not hold
}
