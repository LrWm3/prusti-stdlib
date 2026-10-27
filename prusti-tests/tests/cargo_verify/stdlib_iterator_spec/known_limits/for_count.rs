use prusti_contracts::*;

#[requires(expected == values.len())]
#[ensures(result == expected)]
fn count<T>(values: Vec<T>, expected: usize) -> usize {
    let mut seen = 0usize;
    for _value in values {
        body_invariant!(expected == old(expected));
        body_invariant!(seen <= expected);
        seen += 1;
    }
    seen
}

fn main() {}
