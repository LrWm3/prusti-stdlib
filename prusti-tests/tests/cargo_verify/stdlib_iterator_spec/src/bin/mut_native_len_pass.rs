use prusti_contracts::*;

fn visit<T>(values: &mut [T]) {
    let len = values.len();
    let mut it = values.iter_mut();
    prusti_assert!(it.len() == len);
    match it.next() {
        Some(_) => { prusti_assert!(it.len() + 1 == len); }
        None => { prusti_assert!(len == 0 && it.len() == 0); }
    }
    drop(it);
    prusti_assert!(values.len() == len);
}

#[trusted]
fn main() {
    visit(&mut [String::from("one"), String::from("two")]);
    visit::<u32>(&mut []);
}
