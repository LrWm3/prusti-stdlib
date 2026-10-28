use stdlib_specs::conditional_expiry_without_borrow;
fn main() {
    let _ = conditional_expiry_without_borrow(1usize); // expected-unsupported: conditional expiry requires exactly one borrow wand
}
