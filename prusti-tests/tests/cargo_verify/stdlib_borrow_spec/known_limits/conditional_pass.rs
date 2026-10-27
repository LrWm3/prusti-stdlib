use stdlib_borrow_known_limits::{observe, HasModel};

fn modeled<Value: HasModel>(value: &Value) {
    assert!(observe(value) == 7);
}

fn main() {
    let value = 4usize;
    modeled(&value);
    assert!(observe(&value) == 7);
}
