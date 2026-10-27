use crate::{iter_model::ModeledIterator, *};

// The refinement is limited to the sealed capability; Iterator itself allows
// implementations that resume after returning None.
#[extern_spec(core::iter)]
trait Iterator {
    #[trusted]
    #[refine_spec(where Self: ModeledIterator, [
        ensures(match &result {
            Some(value) => old(self.remaining()).len() > Int::from(0)
                && *value === *old(self.remaining())[0]
                && self.remaining() == old(self.remaining())[1..],
            None => old(self.remaining()).len() == Int::from(0)
                && self.remaining() == old(self.remaining()),
        })
    ])]
    fn next(&mut self) -> Option<Self::Item>;
}

// The sealed models represent exactly the elements still yielded by these
// concrete iterators. Other ExactSizeIterator implementations need not expose
// a sequence model.
#[extern_spec(core::iter)]
trait ExactSizeIterator {
    #[trusted]
    #[refine_spec(where Self: ModeledIterator, [
        ensures(Int::from(result) == self.remaining().len())
    ])]
    fn len(&self) -> usize;
}
