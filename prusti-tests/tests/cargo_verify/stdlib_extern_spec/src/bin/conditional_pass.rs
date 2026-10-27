use prusti_contracts::*;
use stdlib_specs::{
    conditional_identity, conditional_observe, conditional_replace, ConditionalModel,
};

fn modeled<Value: ConditionalModel>(value: &Value) {
    let result = conditional_observe(value);
    prusti_assert!(result == 7);
}

fn main() {
    let marker = 1usize;
    modeled(&marker);
    let result = conditional_observe(&marker);
    prusti_assert!(result == 7);
    let mut value = 4usize;
    let previous = conditional_replace(&mut value, &marker);
    prusti_assert!(previous == 4 && value == 9);
    value = 6;
    let next = conditional_replace(&mut value, &marker);
    prusti_assert!(next == 6 && value == 9);
    let echoed = conditional_identity(5usize);
    prusti_assert!(echoed == 5);
}
