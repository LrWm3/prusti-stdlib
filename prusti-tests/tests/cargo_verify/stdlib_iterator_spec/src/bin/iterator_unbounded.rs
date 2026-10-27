use prusti_contracts::*;

fn arbitrary<I: Iterator>(iterator: &mut I) {
    let first = iterator.next();
    if let None = first {
        let second = iterator.next();
        prusti_assert!(matches!(second, None)); // expected-error: assertion might not hold
    }
}

fn main() {}
