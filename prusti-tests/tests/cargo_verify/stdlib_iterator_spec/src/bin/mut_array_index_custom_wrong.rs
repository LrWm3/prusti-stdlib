use prusti_contracts::*;
use core::ops::{Index, IndexMut};

struct Custom;
#[refine_trait_spec]
impl Index<Custom> for [u32] {
    type Output = [u32];
    #[trusted]
    fn index(&self, _: Custom) -> &[u32] { &self[..1] }
}
#[refine_trait_spec]
impl IndexMut<Custom> for [u32] {
    #[trusted]
    fn index_mut(&mut self, _: Custom) -> &mut [u32] { &mut self[..1] }
}

fn main() {
    let mut values = [3u32, 7u32];
    let slice = &mut values[Custom];
    prusti_assert!(slice.len() == 2); // expected-error: assertion might not hold
}
