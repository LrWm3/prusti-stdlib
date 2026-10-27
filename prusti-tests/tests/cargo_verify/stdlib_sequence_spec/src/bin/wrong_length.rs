use prusti_contracts::*;

fn wrong<T>(value: &[T]) {
    prusti_assert!(Seq::from_slice(value).len() == Int::from(0)); // expected-error: assertion might not hold
}

fn main() {}
