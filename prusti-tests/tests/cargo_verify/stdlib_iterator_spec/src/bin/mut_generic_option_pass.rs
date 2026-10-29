use prusti_contracts::*;
#[ensures(result === old(value))]
fn identity<T>(value: T) -> T {
    value
}
fn visit(value: &mut u32) {
    let output = identity(Some(value));
    match output {
        Some(output) => {
            *output = 7;
            prusti_assert!(*output == 7);
        }
        None => {
            prusti_assert!(false);
        }
    }
}
#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut value);
    assert_eq!(value, 7);
}
