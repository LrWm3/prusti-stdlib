use prusti_contracts::*;

fn wrong() {
    let _iterator = for_iterator::<std::vec::IntoIter<u8>>(); // expected-unsupported: cannot encode method body `wrong`: `prusti_contracts` builtin Spec(ForIterator) cannot be used in executable code
}

fn main() {}
