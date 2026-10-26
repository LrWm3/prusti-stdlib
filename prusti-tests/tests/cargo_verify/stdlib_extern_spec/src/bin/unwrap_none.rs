extern crate stdlib_specs;

fn main() {
    let _ = None::<u32>.unwrap(); // expected-error: precondition might not hold
}
