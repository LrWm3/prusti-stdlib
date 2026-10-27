use prusti_contracts::*;

// Purity belongs to the selected implementation, not every Read implementor.
trait Read {
    fn read(&self) -> usize;
}

struct Value {
    number: usize,
}

#[refine_trait_spec]
impl Read for Value {
    #[pure]
    fn read(&self) -> usize {
        self.number
    }
}

#[pure]
fn observe(value: &Value) -> usize {
    value.read()
}

fn main() {
    let value = Value { number: 13 };
    assert!(value.read() == 13);
    prusti_assert!(observe(&value) == 13);
}
