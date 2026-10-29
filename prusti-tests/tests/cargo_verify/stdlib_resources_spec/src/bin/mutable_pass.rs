use prusti_contracts::*;
use resource_specs::twice;

#[requires(*value == 1)]
#[ensures(*value == 7)]
fn visit(value: &mut u32) {
    let output = twice(value);
    prusti_assert!(*output == 1);
    *output = 7;
    prusti_assert!(*output == 7);
}

fn main() {
    let mut value = 1;
    visit(&mut value);
    prusti_assert!(value == 7);
}
