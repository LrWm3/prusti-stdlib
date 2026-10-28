use prusti_contracts::*;

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { value }

#[requires(*value == 0)]
fn visit(value: &mut u32, choice: bool) {
    let borrowed = if choice { borrow(value) } else { borrow(value) };
    let mut i = 0u32;
    while i < 3 {
        body_invariant!(i < 3 && *borrowed == i);
        *borrowed += 1;
        i += 1;
    }
    prusti_assert!(*value == 3);
}

fn main() {
    visit(&mut 0, true);
    visit(&mut 0, false);
}
