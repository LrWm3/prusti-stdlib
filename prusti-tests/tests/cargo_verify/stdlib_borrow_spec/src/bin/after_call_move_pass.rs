use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32, count: u32 }
impl<'a> Holder<'a> {
    #[after_expiry(after_call(self.count) == 7)]
    fn next(&mut self) -> Option<&'a mut u32> { self.count = 7; None }
}

fn visit(value: &mut u32) {
    let mut holder = Holder { value, count: 1 };
    let result = holder.next();
    let mut moved = holder;
    moved.count = 9;
    if let Some(result) = result { *result = 8; }
    prusti_assert!(moved.count == 9);
    drop(moved);
}

fn main() { let mut value = 1; visit(&mut value); }
