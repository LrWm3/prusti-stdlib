use prusti_contracts::*;

fn visit() {
    let first = ghost! { 1u32 };
    let second = ghost! { *first + 1 };
    prusti_assert!(*second == 2);
}

fn main() { visit(); }
