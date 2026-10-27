use prusti_contracts::*;
struct Wrapper<T> {
    value: T,
}

// PR #1573 also excludes deep snapshots through a generic field.
#[pure]
fn read(value: Wrapper<&mut i32>) -> i32 {
    *value.value // expected-unsupported: cannot encode function body `read`: dereference of the mutable reference `&mut i32` in pure code
}
#[requires(read(value) == 0)]
fn client(value: Wrapper<&mut i32>) {}
fn main() {}
