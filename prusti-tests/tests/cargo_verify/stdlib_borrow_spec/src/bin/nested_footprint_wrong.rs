use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32 }
impl<'a> Holder<'a> {
    // The caller must consider both variants despite this implementation.
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

fn wrong(value: &mut u32) {
    let mut holder = Holder { value };
    if let Some(result) = holder.next() {
        *result = 7;
        prusti_assert!(false); // expected-error: assertion might not hold
    }
}
fn main() {}
