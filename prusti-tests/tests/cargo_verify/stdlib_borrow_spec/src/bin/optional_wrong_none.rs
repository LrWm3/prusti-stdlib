use prusti_contracts::*;

fn main() {
    let mut value = 4usize;
    match borrow_specs::optional(&mut value, false) {
        Some(_) => {
            prusti_assert!(false);
        }
        None => {
            prusti_assert!(value == 9); // expected-error: assertion might not hold
        }
    }
}
