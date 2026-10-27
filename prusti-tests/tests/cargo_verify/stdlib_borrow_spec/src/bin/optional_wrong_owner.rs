use prusti_contracts::*;

fn main() {
    let mut value = 4usize;
    match borrow_specs::optional(&mut value, true) {
        Some(borrowed) => *borrowed = 9,
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(value == 4); // expected-error: assertion might not hold
}
