use prusti_contracts::*;

fn main() {
    let mut first = 1;
    let mut second = 2;
    let (a, b) = resource_specs::separate(&mut first, &mut second);
    *a = 7;
    prusti_assert!(first == 7);
    *b = 8;
    prusti_assert!(false); // expected-error: assertion might not hold
}
