use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut dest = make(5);
    let mut src = make(8);
    collection_specs::concatenate(&mut dest, &mut src);
    prusti_assert!(src.len() == 8); // expected-error: assertion might not hold
}
