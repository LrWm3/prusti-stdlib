use prusti_contracts::*;

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut third = 13usize;
    let mut current = &mut first;
    let oldest = std::mem::replace(&mut current, &mut second);
    let middle = std::mem::replace(&mut current, &mut third);
    *oldest = 7;
    *middle = 11;
    *current = 17;
    let oldest_value = *oldest;
    let middle_value = *middle;
    let current_value = *current;
    prusti_assert!(oldest_value == 7);
    prusti_assert!(middle_value == 11);
    prusti_assert!(current_value == 17);
    prusti_assert!(first == 7);
    prusti_assert!(second == 11);
    prusti_assert!(third == 13); // expected-error: assertion might not hold
}
