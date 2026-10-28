use prusti_contracts::*;

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 {
    value
}

#[requires(*value == 0)]
fn visit(value: &mut u32, choice: bool, mut stop: bool, count: u32) {
    let mut i = 0u32;
    while i < count {
        body_invariant!(i < count && *value == i);
        let borrowed = if choice { borrow(value) } else { borrow(value) };
        i += 1;
        if stop {
            *borrowed = i;
            break;
        }
        *borrowed = i;
        stop = true;
    }
    prusti_assert!(*value == i);
}

fn main() {
    visit(&mut 0, true, true, 0);
    visit(&mut 0, true, true, 4);
    visit(&mut 0, false, false, 4);
}
