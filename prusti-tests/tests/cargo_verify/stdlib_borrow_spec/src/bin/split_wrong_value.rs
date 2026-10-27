use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3, 4];
    let (left, right) = values.split_at_mut(2);
    left.swap(0, 1);
    right.swap(0, 1);
    prusti_assert!(left[0] == 2 && right[0] == 4);
    prusti_assert!(values[0] == 1); // expected-error: assertion might not hold
}
