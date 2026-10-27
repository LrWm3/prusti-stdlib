use prusti_contracts::*;
use stdlib_specs::{Borrowed, BorrowedArray, ReadRef, ReadSize, Reborrow};
fn concrete<'a, T>(value: &'a T) {
    let borrowed = Borrowed { value };
    let result = borrowed.get();
    prusti_assert!(result === value);
}
fn const_generic<'a, const N: usize>(value: &'a [u8; N]) {
    let borrowed = BorrowedArray { value };
    let result = borrowed.size();
    prusti_assert!(result == N);
}
fn shortened<'a>(value: &'a u8) {
    let borrowed = Borrowed { value };
    let result = borrowed.reborrow();
    prusti_assert!(*result == *value);
}
fn main() {
    concrete(&3u8);
    const_generic(&[1u8, 2]);
    shortened(&3u8);
}
