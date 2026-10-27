use borrow_specs::{Borrowed, Cell};

fn modify_abstract<Value: Borrowed>(cell: &mut Value) {
    *cell.borrow() = 11;
}

fn main() {
    let mut cell = Cell { value: 4 };
    let selected = cell.borrow();
    *selected = 9;
    assert!(cell.observe() == 9);
    modify_abstract(&mut cell);
}
