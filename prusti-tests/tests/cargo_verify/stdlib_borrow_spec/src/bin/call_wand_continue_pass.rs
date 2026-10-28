use prusti_contracts::*;

#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 {
    value
}

#[requires(*value == 0)]
fn visit(value: &mut u32, mut skip: bool, count: u32) {
    let mut i = 0u32;
    while i < count {
        body_invariant!(i < count && *value == i);
        if skip {
            *value += 1;
            i += 1;
            skip = false;
            continue;
        }
        let borrowed = if i % 2 == 0 {
            borrow(value)
        } else {
            borrow(value)
        };
        *borrowed += 1;
        i += 1;
        skip = true;
    }
    prusti_assert!(*value == count);
}

fn main() {
    visit(&mut 0, true, 0);
    visit(&mut 0, true, 4);
    visit(&mut 0, false, 4);
}
