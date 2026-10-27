use prusti_contracts::*;

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

// Knowing one pure implementor cannot make an abstract trait call pure.
#[pure]
fn observe<T: Read>(value: &T) -> usize {
    value.read() // expected-unsupported: cannot encode function body `observe`: call to impure function `Read::read` in pure code
}

#[requires(observe(value) == 0)]
fn client<T: Read>(value: &T) {}

fn main() {}
