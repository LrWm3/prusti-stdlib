use prusti_contracts::*;

#[requires(start < values.len())]
#[ensures(Seq::from_slice(&*values)[..start] == old(Seq::from_slice(&*values))[..start])]
fn replace(values: &mut [usize], start: usize, replacement: usize) {
    let part = &mut values[start..];
    part[0] = replacement;
}

fn main() {}
