#![feature(allocator_api)]

use prusti_contracts::*;

#[extern_spec]
impl<T, A: std::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[ensures(self.len() == old(self.len()))]
    fn reserve(&mut self, additional: usize);
}

#[extern_spec]
impl<T, A: std::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[ensures(self.len() == old(self.len()))]
    fn reserve(&mut self, additional: usize); //~ ERROR: duplicate specification for std::vec::Vec::<T, A>::reserve
}

fn main() {}
