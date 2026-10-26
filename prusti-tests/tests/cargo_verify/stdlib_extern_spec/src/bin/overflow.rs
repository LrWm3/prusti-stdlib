extern crate stdlib_specs;

pub fn missing_overflow_precondition(value: u8) -> u8 {
    value + 1 // expected-error: operation may overflow
}

fn main() {}
