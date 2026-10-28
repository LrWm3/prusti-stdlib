use prusti_contracts::*;

fn visit(values: &mut [&mut u32; 2]) {
    let untouched = *values[1];
    *values[0] = 7;
    let updated = *values[0];
    let framed = *values[1];
    prusti_assert!(updated == 8); // expected-error: assertion might not hold
    prusti_assert!(framed == untouched);
}

fn local() {
    let mut value = 1;
    let mut refs = [&mut value];
    *refs[0] = 7;
    let updated = value;
    prusti_assert!(updated == 7);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second]);
    assert_eq!((first, second), (7, 2));
    local();
}
