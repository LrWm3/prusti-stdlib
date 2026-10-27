use collection_specs::{make, view};
use prusti_contracts::*;

#[requires(dest.len() == usize::MAX)]
#[requires(src.len() > 0)]
fn append(dest: &mut Vec<()>, src: &mut Vec<()>) {
    dest.append(src); // expected-error: precondition might not hold
}
fn main() {}
