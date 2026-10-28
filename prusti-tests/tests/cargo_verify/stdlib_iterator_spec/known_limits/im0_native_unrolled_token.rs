use prusti_contracts::*;

struct Token { value: u32 }

#[requires(values.len() == 2)]
#[ensures(values[0].value == 7 && values[1].value == 7)]
fn visit(values: &mut [Token]) {
    let mut it = values.iter_mut();
    match it.next() {
        Some(value) => value.value = 7,
        None => { prusti_assert!(false); }
    }
    match it.next() {
        Some(value) => value.value = 7,
        None => { prusti_assert!(false); }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [Token { value: 1 }, Token { value: 2 }]); }
