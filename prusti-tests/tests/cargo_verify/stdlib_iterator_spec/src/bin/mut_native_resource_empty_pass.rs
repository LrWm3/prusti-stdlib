use prusti_contracts::*;
#[requires(values.len() == 0)]
fn visit(values: &mut [&mut u32]) {
    let mut it = values.iter_mut();
    match it.next() { Some(_) => { prusti_assert!(false); }, None => () }
    drop(it);
    prusti_assert!(values.len() == 0);
}
#[trusted]
fn main() { visit(&mut []); }
