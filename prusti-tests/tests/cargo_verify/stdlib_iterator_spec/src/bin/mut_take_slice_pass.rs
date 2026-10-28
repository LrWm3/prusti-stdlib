use prusti_contracts::*;

fn take<'a>(slot: &mut Option<&'a mut [u32]>) -> Option<&'a mut [u32]> {
    core::mem::replace(slot, None)
}

#[requires(values.len() == 1)]
fn visit(values: &mut [u32]) {
    let mut slot = Some(&mut *values);
    let result = take(&mut slot);
    // The slot can die before the transferred borrow; it must not be needed
    // to restore the source when the result expires.
    drop(slot);
    if let Some(slice) = result {
        if !slice.is_empty() {
            slice[0] = 7;
        }
    }
    prusti_assert!(values.len() == 1);
}

fn main() {
    let mut values = [1];
    visit(&mut values);
}
