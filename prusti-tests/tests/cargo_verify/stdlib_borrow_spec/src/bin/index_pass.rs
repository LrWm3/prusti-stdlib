use prusti_contracts::*;

#[requires(index < values.len())]
#[ensures(values[index] == replacement)]
fn assign_slice(values: &mut [usize], index: usize, replacement: usize) {
    values[index] = replacement;
}

#[requires(index < values.len())]
#[ensures(values[index] == replacement)]
fn assign_vec(values: &mut Vec<usize>, index: usize, replacement: usize) {
    values[index] = replacement;
}

fn main() {
    let mut slice = [1usize, 2, 3];
    assign_slice(&mut slice, 1, 7);
    assert!(slice[1] == 7);
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    assign_vec(&mut values, 1, 7);
    assert!(values[1] == 7);
}
