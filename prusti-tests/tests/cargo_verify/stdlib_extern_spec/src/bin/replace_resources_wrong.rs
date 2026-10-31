use prusti_contracts::*;

#[ensures(result.0 === first)]
#[ensures(result.1 === second)]
fn exchange<T>(first: T, second: T) -> (T, T) {
    let mut slot = first;
    let previous = stdlib_specs::replace_owned(&mut slot, second);
    (previous, slot)
}

fn main() {
    let mut first = 1u32;
    let mut second = 2u32;
    let (previous, current) = exchange(&mut first, &mut second);
    *previous = 13;
    *current = 29;
    prusti_assert!(*previous == 13);
    prusti_assert!(*current == 29);
    prusti_assert!(*previous == *current); // expected-error: assertion might not hold
}
