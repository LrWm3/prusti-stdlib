use prusti_contracts::*;
use resource_specs::{Container, identity_container};

fn main() {
    let mut number = 1;
    let returned = identity_container(Container { value: &mut number });
    prusti_assert!(*returned.value == 1);
    *returned.value = 7;
    prusti_assert!(number == 7);
    prusti_assert!(false); // expected-error: assertion might not hold
}
