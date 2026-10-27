use prusti_contracts::*;

fn main() {
    let mut values = [1usize, 2, 3];
    match values.get_mut(0..2) {
        Some(part) => part[1] = 7,
        None => {
            prusti_assert!(false);
        }
    }
}
