use prusti_contracts::*;

fn take<'a>(slot: &mut Option<&'a mut [u32]>) -> Option<&'a mut [u32]> {
    core::mem::replace(slot, None)
}

fn wrong(values: &mut [u32]) {
    let mut slot = Some(values);
    let result = take(&mut slot);
    drop(slot);
    if let Some(slice) = result {
        let _ = slice.len();
        prusti_assert!(false); // expected-error: assertion might not hold
    }
}

fn main() {}
