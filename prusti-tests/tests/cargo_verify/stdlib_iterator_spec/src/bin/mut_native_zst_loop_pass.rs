use prusti_contracts::*;

fn visit(values: &mut [()]) {
    let len = values.len();
    for value in &mut *values {
        *value = ();
    }
    prusti_assert!(values.len() == len);
    if !values.is_empty() {
        values[0] = ();
    }
}

#[trusted]
fn main() {
    visit(&mut [(), (), ()]);
    visit(&mut []);
}
