use prusti_contracts::*;
fn main() {
    let mut values = [4u32, 11, 15];
    let mut index = 0;
    let selected = &mut values[index];
    index = 1;
    *selected = 7;
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[index] == 11);
    prusti_assert!(values[2] == 15);
}
