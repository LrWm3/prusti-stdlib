use iterator_specs::count_for;
use prusti_contracts::*;

struct Token(usize);

fn generic_count<T>(values: Vec<T>) {
    let length = values.len();
    let result = count_for(values, length);
    prusti_assert!(result == length);
}

// Exercise control flow and evaluation order in ordinary Rust at runtime.
#[trusted]
fn runtime_behavior() {
    let mut evaluations = 0;
    let mut observed = Vec::new();
    prusti_for!(iterator, 'outer: for value in {
        evaluations += 1;
        [Token(1), Token(2), Token(3), Token(4)]
    } => {
        if value.0 == 2 {
            continue 'outer;
        }
        if value.0 == 4 {
            break 'outer;
        }
        observed.push(value.0);
    });
    assert_eq!(evaluations, 1);
    assert_eq!(observed, [1, 3]);

    let mut count = 0;
    prusti_for!(iterator, for _value in [0u8; 0] => {
        count += 1;
    });
    assert_eq!(count, 0);
}

fn main() {
    runtime_behavior();
    generic_count(Vec::<Token>::new());
}
