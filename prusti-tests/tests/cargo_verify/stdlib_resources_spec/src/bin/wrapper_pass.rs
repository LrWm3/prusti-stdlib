use prusti_contracts::*;
use resource_specs::{identity, References};

#[requires(*first == 1 && *second == 2)]
#[ensures(*first == 7 && *second == 8)]
fn visit(first: &mut u32, second: &mut u32) {
    let output = identity(References { first, second });
    prusti_assert!(*output.first == 1 && *output.second == 2);
    *output.first = 7;
    *output.second = 8;
}

fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut first, &mut second);
    prusti_assert!(first == 7 && second == 8);
}
