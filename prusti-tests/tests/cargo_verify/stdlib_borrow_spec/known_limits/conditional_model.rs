use prusti_contracts::*;

pub trait HasModel {}

impl HasModel for usize {}

#[trusted]
#[refine_spec(where Value: HasModel, [
    requires(input > 0),
    ensures(result == 7),
])]
pub fn conditional_pre<Value>(_value: &Value, input: usize) -> usize {
    if input > 0 {
        7
    } else {
        0
    }
}
