use prusti_contracts::*;

#[ensures(result == items.len())]
fn tally<T>(items: &[T]) -> usize {
    let mut cursor = items.iter();
    let mut number = 0;
    loop {
        match cursor.next() {
            Some(_) => number += 1,
            None => break,
        }
    }
    number
}

struct Payload { value: u32 }

fn main() {
    let data = [Payload { value: 4 }, Payload { value: 8 }];
    assert_eq!(tally(&data), 2);
    assert_eq!(tally::<u32>(&[]), 0);
    assert_eq!(tally(&[1u32, 2, 3]), 3);
    assert_eq!(tally(&[(), (), ()]), 3);
}
