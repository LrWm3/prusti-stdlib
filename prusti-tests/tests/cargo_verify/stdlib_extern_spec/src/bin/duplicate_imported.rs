extern crate stdlib_specs;

use prusti_contracts::*;

#[extern_spec]
impl<T> Option<T> {
    #[trusted]
    pub fn unwrap(self) -> T; // expected-incorrect: duplicate specification for `std::option::Option::<T>::unwrap` from crate `duplicate_imported` and `stdlib_specs`
}

fn main() {}
