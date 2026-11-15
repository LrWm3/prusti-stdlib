use prusti_contracts::*;
#[path = "../support/native_mut.rs"]
mod native;
#[requires(values.len() == 1 && values[0] == 1)]
#[ensures(values[0] == 1)]
fn visit(values: &mut [u32]) {
    let it = values.iter_mut();
    drop(it);
}
fn main() {}
