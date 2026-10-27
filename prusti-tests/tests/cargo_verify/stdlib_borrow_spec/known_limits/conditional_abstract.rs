use stdlib_borrow_known_limits::observe;

fn abstract_value<Value>(value: &Value) {
    assert!(observe(value) == 7);
}

fn main() {}
