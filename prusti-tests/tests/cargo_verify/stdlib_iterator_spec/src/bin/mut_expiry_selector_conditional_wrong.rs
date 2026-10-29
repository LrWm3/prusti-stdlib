use prusti_contracts::*;

#[assert_on_expiry(first => *result.0 == 7, *first == 7)]
#[after_expiry(second => *second == before_expiry(*result.1))]
fn separate<'a, 'b>(first: &'a mut u32, second: &'b mut u32) -> (&'a mut u32, &'b mut u32) {
    (first, second)
}

fn main() {
    let mut first = 1;
    let mut second = 2;
    let (a, b) = separate(&mut first, &mut second);
    *a = 6;
    prusti_assert!(first == 7); // expected-error: obligation might not hold on borrow expiry
    *b = 8;
    prusti_assert!(second == 8);
}
