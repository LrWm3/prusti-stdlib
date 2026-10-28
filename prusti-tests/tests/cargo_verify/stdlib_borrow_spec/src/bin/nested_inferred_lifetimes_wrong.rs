use prusti_contracts::*;

struct Holder<'a, T> { value: &'a mut T }
impl<'a, T> Holder<'a, T> {
    #[after_expiry(Ghost::new_ref(&*self.value) == before_expiry(Ghost::new_ref(&*result)))]
    fn take(self) -> &'a mut T { self.value }
}

fn wrong(value: &mut u32) {
    let holder = Holder { value: &mut *value };
    let result = holder.take();
    *result = 7;
    prusti_assert!(*value == 8); // expected-error: assertion might not hold
}
fn main() {}
