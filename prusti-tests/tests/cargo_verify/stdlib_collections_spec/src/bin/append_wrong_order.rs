use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut dest = make(5);
    let mut src = make(8);
    dest.append(&mut src);
    prusti_assert!(dest[5].value == 5); // expected-error: assertion might not hold
}
