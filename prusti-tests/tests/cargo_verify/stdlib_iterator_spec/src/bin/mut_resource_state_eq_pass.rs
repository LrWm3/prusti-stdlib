use prusti_contracts::*;

#[requires(*first == 1 && *second == 1)]
#[ensures(*first == 2 && *second == 1)]
fn distinguish(first: &mut u32, second: &mut u32) {
    prusti_assert!(resource_state(&first) == resource_state(&second));
    *first = 2;
    prusti_assert!(resource_state(&first) != resource_state(&second));
}

fn main() {
    let (mut first, mut second) = (1, 1);
    distinguish(&mut first, &mut second);
    assert_eq!((first, second), (2, 1));
}
