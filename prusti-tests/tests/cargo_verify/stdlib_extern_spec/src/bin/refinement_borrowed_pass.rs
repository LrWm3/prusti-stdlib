use prusti_contracts::*;
use stdlib_specs::{refined_advance, RefinedFinite, RefinedOnceRef};

fn check<'a, T>(value: &'a T) {
    let mut iterator = RefinedOnceRef {
        value,
        available: true,
    };
    let before = ghost! { iterator.remaining() };
    let first = refined_advance(&mut iterator);
    match first {
        Some(item) => {
            prusti_assert!(item === value);
            prusti_assert!(iterator.remaining() == (*before)[1..]);
        }
        None => {
            prusti_assert!(false);
        }
    }
    let second = refined_advance(&mut iterator);
    prusti_assert!(matches!(second, None));
}

struct Token(u32);
fn main() {
    let value = Token(7);
    check(&value);
}
