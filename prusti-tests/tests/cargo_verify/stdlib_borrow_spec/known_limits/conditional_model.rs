use prusti_contracts::*;

pub trait HasModel {}

impl HasModel for usize {}

#[trusted]
#[refine_spec(where Value: HasModel, [ensures(result == 7)])]
pub fn observe<Value>(_value: &Value) -> usize {
    7
}
