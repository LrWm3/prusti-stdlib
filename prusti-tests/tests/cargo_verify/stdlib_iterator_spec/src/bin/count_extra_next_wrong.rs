use prusti_contracts::*;

#[ensures(result == values.len())] // expected-error: postcondition might not hold
fn count<T>(values: &[T]) -> usize {
    let mut it = values.iter();
    let mut seen = 0;
    loop {
        match it.next() {
            Some(_) => { seen += 1; let _ = it.next(); },
            None => break,
        }
    }
    seen
}

fn main() {}
