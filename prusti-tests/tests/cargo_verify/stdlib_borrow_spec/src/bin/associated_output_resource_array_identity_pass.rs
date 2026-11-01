use prusti_contracts::*;

#[ensures(result === values)]
fn identity_array<T, const N: usize>(values: [T; N]) -> [T; N] {
    values
}

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut selected = identity_array([&mut first, &mut second]);
    *selected[0] = 7;
    *selected[1] = 11;
    let first_value = *selected[0];
    let second_value = *selected[1];
    prusti_assert!(first_value == 7);
    prusti_assert!(second_value == 11);
    prusti_assert!(first == 7);
    prusti_assert!(second == 11);
}
