extern crate stdlib_specs;

pub fn missing_precondition<T>(value: Option<T>) -> T {
    value.unwrap() // expected-error: precondition might not hold
}

fn main() {}
