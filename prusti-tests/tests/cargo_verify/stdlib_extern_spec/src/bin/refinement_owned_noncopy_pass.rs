use prusti_contracts::*;
use stdlib_specs::{refined_advance, RefinedOnceOwned, RefinedToken};

fn main() {
    let mut iterator = RefinedOnceOwned {
        value: Some(RefinedToken { value: 7 }),
    };
    let first = refined_advance(&mut iterator);
    match first {
        Some(value) => assert_eq!(value.value, 7),
        None => {
            prusti_assert!(false);
        }
    }
    let second = refined_advance(&mut iterator);
    prusti_assert!(matches!(second, None));
}
