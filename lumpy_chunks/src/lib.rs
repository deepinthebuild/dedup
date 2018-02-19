use std::ops::Fn;
use std::mem;

pub trait LumpyChunks {
    type Contents;
    fn lumpy_chunks<P>(&self, rough_size: usize, break_point_finder: P) -> LumpyChunksIter<Self::Contents, P> where for<'r> P: Fn(&'r [Self::Contents],) -> Option<usize>;
}

pub struct LumpyChunksIter<'a, T: 'a, P> where for<'r> P : Fn(&'r [T]) -> Option<usize> {
    slice: &'a [T],
    rough_size: usize,
    break_point_finder: P
}


impl<T> LumpyChunks for [T] {
    type Contents = T;
    fn lumpy_chunks<P>(&self, rough_size: usize, break_point_finder: P) -> LumpyChunksIter<Self::Contents, P> where for<'r> P: Fn(&'r [T],) -> Option<usize> {
        LumpyChunksIter{
            slice: self,
            rough_size,
            break_point_finder,
        }
    }

}

impl<'a, T: 'a, P> Iterator for LumpyChunksIter<'a, T, P> where P: Fn(&[T]) -> Option<usize>  {
    type Item = &'a [T];
    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None
        }
        if self.slice.len() <= self.rough_size {
            return Some(mem::replace(&mut self.slice, &[]))
        }

        if let Some(u) = (self.break_point_finder)(&self.slice[self.rough_size..]) {
            let (head, tail) = self.slice.split_at(self.rough_size + u);
            self.slice = tail;
            return Some(head)
        } else {
            return Some(mem::replace(&mut self.slice, &[]))
        }


    }
}

#[cfg(test)]
mod tests {
}
