use prusti_contracts::*;

fn main() {
    let mut value = 4usize;
    match borrow_specs::optional(&mut value, true) {
        Some(borrowed) => {
            assert!(*borrowed == 4);
            *borrowed = 9;
        }
        None => {
            prusti_assert!(false);
        }
    }
    assert!(value == 9);
    match borrow_specs::optional(&mut value, false) {
        Some(_) => {
            prusti_assert!(false);
        }
        None => assert!(value == 9),
    }
}
