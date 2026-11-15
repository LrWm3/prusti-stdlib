use prusti_contracts::*;

#[ensures(result == values.len())] // expected-error: postcondition might not hold
fn count<T>(values: &[T]) -> usize {
    let mut it = values.iter();
    let mut seen = 0;
    loop {
        match it.next() {
            Some(_) => { seen += 1; if seen == 1 { break; } },
            None => break,
        }
    }
    seen
}

fn main() {}
