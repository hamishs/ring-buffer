use crate::RingBuffer;

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

#[cfg(test)]
mod tests {

    use crate::RingBuffer;

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
}
