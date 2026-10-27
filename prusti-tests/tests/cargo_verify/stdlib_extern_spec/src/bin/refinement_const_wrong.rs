use prusti_contracts::*;
use stdlib_specs::{BorrowedArray, ReadSize};
fn wrong<'a, const N: usize>(value: &'a [u8; N]) {
    let borrowed = BorrowedArray { value };
    let result = borrowed.size();
    prusti_assert!(Int::from(result) == Int::from(N) + Int::from(1)); // expected-error: assertion might not hold
}
fn main() {}
