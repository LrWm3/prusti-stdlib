use prusti_contracts::*;

#[after_expiry(forall(|i: usize| i < 1 ==> *value == before_expiry(*result)))]
fn borrow(value: &mut u32) -> &mut u32 { value }

fn visit(value: &mut u32) {
    let borrowed = borrow(value);
    *borrowed = 7;
    prusti_assert!(*value == 7);
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut value);
    assert_eq!(value, 7);
}
