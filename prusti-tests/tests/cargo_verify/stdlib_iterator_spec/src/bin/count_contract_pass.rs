use prusti_contracts::*;

#[ensures(result == values.len())]
fn count<T>(values: &[T]) -> usize {
    let mut it = values.iter();
    let mut seen = 0;
    loop {
        match it.next() {
            Some(_) => seen += 1,
            None => break,
        }
    }
    seen
}

fn main() {
    assert_eq!(count::<u32>(&[]), 0);
    assert_eq!(count(&[1u32, 2, 3]), 3);
    assert_eq!(count(&[(), (), ()]), 3);
}
