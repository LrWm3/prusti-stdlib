use prusti_contracts::*;
use resource_specs::{identity_associated, ResourceSource};

struct Mutable<'a>(core::marker::PhantomData<&'a mut u32>);
impl<'a> ResourceSource for Mutable<'a> {
    type Resource = &'a mut u32;
}

#[requires(*value == 1)]
#[ensures(*value == 7)]
fn visit(value: &mut u32) {
    let output = identity_associated::<Mutable<'_>>(value);
    prusti_assert!(*output == 1);
    *output = 7;
}

fn main() {
    let mut value = 1;
    visit(&mut value);
    prusti_assert!(value == 7);
}
