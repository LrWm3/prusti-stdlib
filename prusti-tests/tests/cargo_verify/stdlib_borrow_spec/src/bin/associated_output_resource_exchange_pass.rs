use prusti_contracts::*;

#[ensures(result.0 === right)]
#[ensures(result.1 === left)]
fn exchange<T>(left: T, right: T) -> (T, T) {
    (right, left)
}

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let (right, left) = exchange(&mut first, &mut second);
    *left = 7;
    *right = 11;
    prusti_assert!(first == 7);
    prusti_assert!(second == 11);
}
