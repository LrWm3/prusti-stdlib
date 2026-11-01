use prusti_contracts::*;

fn take<'a>(slot: &mut Option<&'a mut str>) -> Option<&'a mut str> {
    core::mem::replace(slot, None)
}

fn visit(value: &mut str) {
    let mut slot = Some(value);
    let result = take(&mut slot);
    drop(slot);
    if let Some(_text) = result {
        prusti_assert!(false); // expected-error: assertion might not hold
    }
}

#[trusted]
fn main() {
    let mut bytes = *b"abc";
    visit(core::str::from_utf8_mut(&mut bytes).unwrap());
    let mut empty = [];
    visit(core::str::from_utf8_mut(&mut empty).unwrap());
    assert_eq!(bytes, *b"abc");
}
