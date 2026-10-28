use prusti_contracts::*;

#[requires(values.len() == 1)]
fn visit(values: &mut [Option<&mut u32>]) {
    if let Some(value) = &mut values[0] {
        **value = 7;
    }
}

#[trusted]
fn main() {
    let mut first = 1;
    visit(&mut [Some(&mut first)]);
    assert_eq!(first, 7);
    visit(&mut [None]);
}
