use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let reference = &mut value;
    let initial = ghost! { resource_state(&reference) };
    *reference = 7;
    let current = ghost! { resource_state(&reference) };
    prusti_assert!(*current != *initial);
}
