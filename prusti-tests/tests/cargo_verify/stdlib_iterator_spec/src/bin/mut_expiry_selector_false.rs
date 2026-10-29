use prusti_contracts::*;

#[after_expiry(first => *first == before_expiry(*result.0))]
#[after_expiry(second => *second == before_expiry(*result.1))]
fn separate<'a, 'b>(first: &'a mut u32, second: &'b mut u32) -> (&'a mut u32, &'b mut u32) {
    (first, second)
}

fn main() {
    let mut first = 1;
    let mut second = 2;
    let (a, b) = separate(&mut first, &mut second);
    *a = 7;
    prusti_assert!(first == 7);
    *b = 8;
    prusti_assert!(false); // expected-error: assertion might not hold
}
