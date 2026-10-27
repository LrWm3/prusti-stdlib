use borrow_specs::Borrowed;
use prusti_contracts::*;

fn abstract_client<Value: Borrowed>(cell: &mut Value) {
    let selected = cell.borrow();
    *selected = 9;
    prusti_assert!(cell.observe() == 9); // expected-error: assertion might not hold
}

fn main() {}
