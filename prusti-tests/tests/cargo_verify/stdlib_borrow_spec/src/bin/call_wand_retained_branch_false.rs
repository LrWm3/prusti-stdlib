use prusti_contracts::*;

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 {
    value
}

#[requires(*value == 0)]
fn visit(value: &mut u32, choice: bool, count: u32) {
    let borrowed = if choice { borrow(value) } else { borrow(value) };
    let mut i = 0u32;
    while i < count {
        body_invariant!(i < count && *borrowed == i);
        *borrowed += 1;
        i += 1;
    }
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {
    visit(&mut 0, true, 0);
    visit(&mut 0, true, 3);
    visit(&mut 0, false, 0);
    visit(&mut 0, false, 3);
}
