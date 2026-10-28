use prusti_contracts::*;

fn loan<'a>(value: &'a mut u32, other: &mut u32) -> &'a mut u32 {
    *other = 3;
    value
}

fn main() {
    let mut value = 1;
    let mut other = 0;
    let result = loan(&mut value, &mut other);
    *result = 2;
    prusti_assert!(value == 1); // expected-error: assertion might not hold
}
