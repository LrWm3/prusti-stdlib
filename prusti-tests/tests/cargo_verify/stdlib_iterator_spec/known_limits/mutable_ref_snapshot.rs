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
    #[ensures(forall(|i: usize| i < old(self.len()) ==> &**remaining_refs(&result)[i] === old(&self[i])))]
    fn iter_mut(&mut self) -> IterMut<'_, T>;
}

#[extern_spec]
impl<'a, T> Iterator for IterMut<'a, T> {
    #[trusted]
    #[ensures(match &result {
        Some(value) => old(remaining(&*self)).len() > Int::from(0)
            && &**value === &**old(remaining_refs(&*self))[0]
            && Ghost::new_ref(&**value) == Ghost::new_ref(&*old(remaining(&*self))[0])
            && remaining(&*self) == old(remaining(&*self))[1..],
        None => old(remaining(&*self)).len() == Int::from(0)
            && remaining(&*self) == old(remaining(&*self)),
    })]
    #[ensures(match &result {
        Some(_) => remaining_refs(&*self) == old(remaining_refs(&*self))[1..],
        None => remaining_refs(&*self) == old(remaining_refs(&*self)),
    })]
    fn next(&mut self) -> Option<&'a mut T>;
}


#[requires(values.len() == 2)]
#[requires(values[0] == 1 && values[1] == 2)]
fn visit(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(remaining(&iterator).len() == Int::from(2));
    match iterator.next() {
        Some(value) => { prusti_assert!(*value == 1); *value = 7; }
        None => { prusti_assert!(false); }
    }
    prusti_assert!(remaining(&iterator).len() == Int::from(1));
    match iterator.next() {
        Some(value) => { prusti_assert!(*value == 2); *value = 8; }
        None => { prusti_assert!(false); }
    }
    let next = iterator.next();
    prusti_assert!(matches!(next, None));
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 8);
}
fn main() { let mut values = [1, 2]; visit(&mut values); }
