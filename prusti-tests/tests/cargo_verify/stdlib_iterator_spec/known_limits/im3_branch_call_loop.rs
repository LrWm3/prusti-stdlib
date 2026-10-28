use prusti_contracts::*;

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { value }

#[requires(*value == 0)]
fn visit(value: &mut u32, mut choice: bool) {
    let mut i = 0u32;
    while i < 3 {
        body_invariant!(i < 3 && *value == i);
        let borrowed = if choice { borrow(value) } else { borrow(value) };
        *borrowed += 1;
        i += 1;
        choice = !choice;
        prusti_assert!(*value == i);
    }
    prusti_assert!(*value == 3);
}

fn main() {
    visit(&mut 0, true);
    visit(&mut 0, false);
}
