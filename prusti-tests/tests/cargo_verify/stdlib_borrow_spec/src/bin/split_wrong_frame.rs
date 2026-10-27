use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3, 4, 5, 6];
    let (left, right) = values.split_at_mut(3);
    left.swap(0, 1);
    right.swap(0, 1);
    prusti_assert!(left[0] == 2 && right[0] == 5);
    prusti_assert!(values[2] == 6); // expected-error: assertion might not hold
}
