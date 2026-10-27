use prusti_contracts::*;

trait Update {
    #[requires(*value < u32::MAX)]
    #[ensures(*value == old(*value) + 1)]
    #[ensures(*observation == old(*observation))]
    fn update(value: &mut u32, observation: &u32);
}

struct Increment;

#[refine_trait_spec]
impl Update for Increment {
    #[requires(*value < u32::MAX)]
    #[ensures(*value == old(*value) + 1)]
    #[ensures(*observation == old(*observation))]
    fn update(value: &mut u32, observation: &u32) {
        *value += 1;
    }
}

#[requires(*value < u32::MAX)]
#[ensures(*value == old(*value) + 1)]
fn generic<T: Update>(value: &mut u32, observation: &u32) {
    let before = *observation;
    T::update(value, observation);
    prusti_assert!(*observation == before);
}

fn main() {
    let mut value = 1;
    let observation = 8;
    generic::<Increment>(&mut value, &observation);
    assert_eq!(value, 2);
}
