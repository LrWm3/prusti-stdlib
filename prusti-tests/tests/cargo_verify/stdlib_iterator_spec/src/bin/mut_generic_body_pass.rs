use prusti_contracts::*;
#[ensures(result === old(value))]
fn identity<T>(value: T) -> T { value }
fn reborrow<T>(value: &mut T) -> &mut T { value }
#[requires(values.len() > 0)]
fn head<T>(values: &mut [T]) -> &mut T {
    &mut values[0]
}
#[trusted]
fn main() {}
