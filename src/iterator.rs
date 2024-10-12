//! Iterators for the ring buffer.
use crate::RingBuffer;

/// An iterator over borrowed elements of a ring buffer.
pub struct Iter<'a, T> {
    pub(crate) rb: &'a RingBuffer<T>,
    pub(crate) index: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.rb.len {
            let item = self.rb.get(self.index);
            self.index += 1;
            item
        } else {
            None
        }
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index < self.rb.len {
            let item = self.rb.get(self.rb.len - self.index - 1);
            self.index += 1;
            item
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.rb.len - self.index
    }
}

/// An iterator that moves out of a ring buffer.
///
/// Since the buffer is naturally double ended we don't need any special logic to support
/// iteration.
pub struct IntoIter<T>(RingBuffer<T>);

impl<T> IntoIterator for RingBuffer<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.pop_back()
    }
}

#[cfg(test)]
mod tests {

    use crate::RingBuffer;

    #[test]
    fn test_iter() {
        let mut rb = RingBuffer::<i32>::with_capacity(6);

        // [5, 6, ., 1, 2, 3, 4]
        rb.push_back(5);
        rb.push_back(6);
        rb.push_front(4);
        rb.push_front(3);
        rb.push_front(2);
        rb.push_front(1);

        let values: Vec<i32> = rb.iter().copied().collect();
        assert_eq!(values, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_into_iter() {
        let mut rb = RingBuffer::<i32>::with_capacity(6);

        // [5, 6, ., 1, 2, 3, 4]
        rb.push_back(5);
        rb.push_back(6);
        rb.push_front(4);
        rb.push_front(3);
        rb.push_front(2);
        rb.push_front(1);

        let values: Vec<i32> = rb.into_iter().collect();
        assert_eq!(values, [1, 2, 3, 4, 5, 6]);
    }
}
