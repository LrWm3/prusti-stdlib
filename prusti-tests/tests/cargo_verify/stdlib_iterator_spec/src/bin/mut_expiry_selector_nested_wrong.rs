use prusti_contracts::*;

#[after_expiry(value => Ghost::new_ref(&*value) == before_expiry(Ghost::new_ref(&*result)))]
#[after_expiry(*value => false)] // expected-error: pledge postcondition might not hold
fn nested<'a, 'b>(value: &'a mut &'b mut u32) -> &'a mut &'b mut u32 { value }

#[after_expiry(*value => true)]
fn repeated<'a>(value: &'a mut &'a mut u32) -> &'a mut &'a mut u32 { value }

#[trusted]
fn main() {
    let mut number = 1;
    let mut inner = &mut number;
    **nested(&mut inner) = 7;
    assert_eq!(number, 7);
}
