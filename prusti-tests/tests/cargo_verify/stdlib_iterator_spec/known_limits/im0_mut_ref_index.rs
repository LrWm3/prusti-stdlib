use prusti_contracts::*;

#[requires(values.len() == 2)]
fn visit(values: &mut [&mut u32]) {
    *values[0] = 7;
    prusti_assert!(*values[0] == 7);
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut [&mut first, &mut second]);
    assert_eq!(first, 7);
}
