use prusti_contracts::*;
#[path = "../support/mut_region_full.rs"]
mod model;
use model::Iter;

fn visit(left: &mut [u32], right: &mut [u32], choose_left: bool) {
    // The owner reference is selected inside the method. Capturing its value
    // at method entry would read an uninitialised local.
    let values = if choose_left { left } else { right };
    let mut it = Iter { tail: Some(values) };
    loop {
        // Exercise the automatically generated resource invariant too.
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

#[trusted]
fn runtime_check() {
    let mut left = [1, 2];
    let mut right = [3, 4];
    visit(&mut left, &mut right, true);
    assert_eq!(left, [11, 7]);
    assert_eq!(right, [3, 4]);
    visit(&mut left, &mut right, false);
    assert_eq!(right, [11, 7]);
    visit(&mut [], &mut [], false);
}

fn main() { runtime_check(); }
