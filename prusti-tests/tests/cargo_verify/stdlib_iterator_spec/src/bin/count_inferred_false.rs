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
    prusti_assert!(false); // expected-error: assertion might not hold
    seen
}

fn main() {}
