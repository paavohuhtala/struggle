use tinyvec::{Array, TinyVec};

pub trait TinyVecExt {
    type Item;
    fn into_vec(self) -> Vec<Self::Item>;
}

impl<A: Array> TinyVecExt for TinyVec<A> {
    type Item = A::Item;

    fn into_vec(mut self) -> Vec<Self::Item> {
        self.move_to_the_heap();

        match self {
            TinyVec::Heap(vec) => vec,
            _ => unreachable!(),
        }
    }
}
