use prusti_contracts::*;

struct Changes<'a>(&'a mut u32);
impl<'a> Drop for Changes<'a> {
    fn drop(&mut self) {
        *self.0 = 7;
    }
}

fn main() {
    let mut value = 1;
    drop(Changes(&mut value));
    prusti_assert!(value == 1); // expected-error: assertion might not hold
}
