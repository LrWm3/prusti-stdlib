use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;

struct Token { value: u32 }

#[requires(values.len() == 2)]
#[requires(values[0].value == 1 && values[1].value == 2)]
fn visit(values: &mut [Token]) {
    let mut it = values.iter_mut();
    let first = it.next();
    let second = it.next();
    drop(it);
    match (first, second) {
        (Some(first), Some(second)) => {
            *second = Token { value: 8 };
            prusti_assert!(first.value == 1);
            *first = Token { value: 7 };
        }
        _ => { prusti_assert!(false); }
    }
    prusti_assert!(values[0].value == 7);
    prusti_assert!(values[1].value == 8);
}

#[trusted]
fn main() {
    let mut values = [Token { value: 1 }, Token { value: 2 }];
    visit(&mut values);
    assert_eq!(values[0].value, 7);
    assert_eq!(values[1].value, 8);
}
