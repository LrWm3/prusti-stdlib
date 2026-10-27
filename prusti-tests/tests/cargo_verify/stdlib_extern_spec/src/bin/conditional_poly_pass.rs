use prusti_contracts::*;
use stdlib_specs::conditional_copy_ref;

fn bounded<'a, T: Copy>(value: &'a T) {
    let result = conditional_copy_ref(&value);
    prusti_assert!(result == 7);
}

fn main() {
    bounded(&42u32);
}
