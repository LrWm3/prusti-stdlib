use prusti_contracts::*;
use stdlib_specs::{conditional_borrow, ConditionalModel};

fn generic<M: ConditionalModel>(value: &mut usize, marker: M) {
    *conditional_borrow(value, marker) = 7;
    prusti_assert!(*value == 7);
}
fn main() {
    let mut value = 1;
    generic(&mut value, 1usize);
}
