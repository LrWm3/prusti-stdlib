use core::slice::IterMut;
use prusti_contracts::*;
#[pure]
#[trusted]
pub fn remaining<T>(_iter: &IterMut<'_, T>) -> Seq<T> { Seq::new() }
#[pure]
#[trusted]
pub fn remaining_refs<'a, T>(_iter: &IterMut<'a, T>) -> Seq<&'a mut T> { Seq::new() }
#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[ensures(remaining(&result) == old(Seq::from_slice(&*self)))]
    #[ensures(remaining_refs(&result).len() == Int::from(old(self.len())))]
    #[ensures(forall(|i: usize| i < old(self.len()) ==> mut_ref_shared_same_address(&*remaining_refs(&result)[i], old(&self[i]))))]
    #[after_expiry(self.len() > 0 ==> Ghost::new_ref(&self[0]) == before_expiry(Ghost::new_ref(&**remaining_refs(&result)[0])))]
    fn iter_mut(&mut self) -> IterMut<'_, T>;
}
#[requires(values.len() == 1)]
fn visit(values: &mut [u32]) {
    let iterator = values.iter_mut();
    drop(iterator);
    prusti_assert!(values.len() == 1);
}
fn main() { let mut values = [1]; visit(&mut values); }
