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
    if !vector.is_empty() {
        vector[0] = 11;
        prusti_assert!(vector[0] == 11);
    }
}

#[trusted]
fn main() {
    let mut values = vec![1, 2, 3];
    visit(&mut values);
    assert_eq!(values, [11, 7, 7]);
}
