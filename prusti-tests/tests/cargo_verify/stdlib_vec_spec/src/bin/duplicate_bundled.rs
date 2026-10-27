use prusti_contracts::*;

#[extern_spec]
impl<T> Vec<T> {
    #[trusted]
    fn new() -> Self; // expected-incorrect: duplicate specification for `std::vec::Vec::<T>::new` from crate `duplicate_bundled` and `prusti_contracts`
}

fn main() {}
