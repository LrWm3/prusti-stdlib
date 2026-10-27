use prusti_contracts::*;

#[extern_spec(target_api)]
#[trusted]
#[ensures(*result == *first)]
fn outlives<'left: 'right, 'right>(first: &'left u32, second: &'right u32) -> &'right u32;

#[extern_spec(target_api)]
#[trusted]
#[ensures(*result === *value)]
fn aliased<T>(value: &T) -> &T;

// Rename both nested binders independently.
#[extern_spec(target_api)]
#[trusted]
fn callback(value: for<'input> fn(&'input u32) -> &'input u32)
    -> for<'output> fn(&'output u32) -> &'output u32;

fn main() {
    let first = 7;
    let second = 11;
    assert!(*target_api::outlives(&first, &second) == 7);
    assert!(*target_api::aliased(&first) == 7);
}
