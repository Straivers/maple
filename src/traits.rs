pub trait OutputIter<T> {
    fn push(&mut self, value: T);
}

impl<T> OutputIter<T> for Vec<T> {
    fn push(&mut self, value: T) {
        Vec::push(self, value);
    }
}

pub trait CountingOutputIter<T>: OutputIter<T> {
    fn len(&self) -> usize;
}

impl<T> CountingOutputIter<T> for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}
