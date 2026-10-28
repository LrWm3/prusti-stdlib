use prusti_contracts::*;

fn visit(vector: &mut Vec<u32>) {
    let slice = vector.as_mut_slice();
    let mut iterator = slice.iter_mut();
    loop {
        body_invariant!(true);
        match iterator.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(iterator);
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
