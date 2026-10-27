use prusti_contracts::*;

fn wrong<T>(value: &[T]) {
    prusti_assert!(Seq::from_slice(value).len() == Int::from(value.len()));
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
