use prusti_contracts::*;

struct Borrow<'a>(&'a mut u32);

#[ensures(*value == old(*value))]
fn preserve(value: &mut u32) {
    let initial = *value;
    drop(Borrow(value));
    prusti_assert!(*value == initial);
}

fn main() {
    let mut value = 1;
    preserve(&mut value);
    prusti_assert!(value == 1);
}
