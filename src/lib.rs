//! A growable ring buffer implementation.
//!
//! <https://en.wikipedia.org/wiki/Circular_buffer>
//! <https://stackoverflow.com/questions/49072494/how-does-the-vecdeque-ring-buffer-work-internally>
//! <https://doc.rust-lang.org/nomicon/vec/vec-push-pop.html>
//!
#![allow(dead_code)]
use std::{
    alloc::Layout,
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

mod iterator;
mod traits;
use iterator::Iter;

/// A growable ring buffer.
/// Contains a pointer to the buffer, the allocated capacity, the current length of the buffer and
/// the head index.
#[derive(Debug)]
pub struct RingBuffer<T> {
    ptr: NonNull<T>,
    capacity: usize,
    head: usize,
    len: usize,
}

impl<T> RingBuffer<T> {
    pub fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "ZSTs are not supported.");
        RingBuffer {
            ptr: NonNull::dangling(),
            capacity: 0,
            head: 0,
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(std::mem::size_of::<T>() != 0, "ZSTs are not supported.");
        if capacity == 0 {
            return Self::new();
        }

        let layout = Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };

        let ptr = match NonNull::new(ptr as *mut T) {
            Some(ptr) => ptr,
            None => std::alloc::handle_alloc_error(layout),
        };

        RingBuffer {
            ptr,
            capacity,
            head: 0,
            len: 0,
        }
    }

    /// Return true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get the item at position `index` in the buffer.
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        let idx = (self.head + index) % self.capacity;
        unsafe { Some(&*self.ptr.as_ptr().add(idx)) }
    }

    /**
     * Inserting elements.
     */

    /// Insert an item at the front of the buffer.
    pub fn push_front(&mut self, item: T) {
        if self.is_full() {
            self.grow();
        }

        let index = (self.head + self.capacity - 1) % self.capacity;

        unsafe {
            self.ptr.as_ptr().add(index).write(item);
        }

        self.head = index;
        self.len += 1;
    }

    /// Insert an item at the end of the buffer.
    pub fn push_back(&mut self, item: T) {
        if self.is_full() {
            self.grow();
        }

        let index = (self.head + self.len) % self.capacity;

        unsafe {
            self.ptr.as_ptr().add(index).write(item);
        }
        self.len += 1;
    }

    /**
     * Removing elements.
     */

    /// Remove the element from the front if there is one and return it.
    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let item = unsafe { self.ptr.as_ptr().add(self.head).read() };
        self.head = (self.head + 1) % self.capacity;
        self.len -= 1;

        Some(item)
    }

    /// Remove the element from the back if there is one and return it.
    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let idx = (self.head + self.len - 1) % self.capacity;
        let item = unsafe { self.ptr.as_ptr().add(idx).read() };
        self.len -= 1;

        Some(item)
    }

    /**
     * Memory layout.
     */

    /// Return true if the buffer is contiguous in memory.
    pub fn is_contiguous(&self) -> bool {
        self.head + self.len <= self.capacity
    }

    /// Return the buffer a pair of slices.
    ///
    /// Two are required because the buffer may wrap around the end of the allocated memory.
    /// The front of the buffer is always the first slice and the back is always the second.
    /// If the buffer is contiguous then the second slice will be empty.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        if self.is_contiguous() {
            let slice = unsafe { from_raw_parts(self.ptr.as_ptr().add(self.head), self.len) };
            return (slice, &[]);
        }

        let top = self.capacity - self.head;
        let bottom = self.len - top;

        let first = unsafe { from_raw_parts(self.ptr.as_ptr().add(self.head), top) };
        let second = unsafe { from_raw_parts(self.ptr.as_ptr(), bottom) };
        (first, second)
    }

    /// Return the buffer as a pair of mutable slices.
    pub fn as_mut_slices(&self) -> (&mut [T], &mut [T]) {
        if self.is_contiguous() {
            let slice = unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), self.len) };
            return (slice, &mut []);
        }

        let top = self.capacity - self.head;
        let bottom = self.len - top;

        let first = unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), top) };
        let second = unsafe { from_raw_parts_mut(self.ptr.as_ptr(), bottom) };
        (first, second)
    }

    /// Restructure the buffer so it is contiguous in memory.
    ///
    /// Returns a mutable reference to the buffer as a slice.
    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.is_contiguous() {
            return unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), self.len) };
        }
        //                       H
        // [o, o, ...., ., ., ., o, o, o]
        let top = self.capacity - self.head;
        let bottom = self.head + self.len - self.capacity;
        let spare = self.capacity - self.len;

        if bottom <= spare {
            // Can copy the bottom part up into the spare space.
            // [., ., ..., o, o, o, o, o, ., ., ....]
            unsafe {
                self.ptr
                    .as_ptr()
                    .add(self.head)
                    .copy_to(self.ptr.as_ptr().add(bottom), top);
                self.ptr
                    .as_ptr()
                    .copy_to(self.ptr.as_ptr().add(bottom + top), bottom);
            }
            self.head = bottom;
            return unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), self.len) };
        } else if top <= spare {
            // Can copy the top part down into the spare space.
            // [o, o, o, o, o, ., ., ..., ., ., .]
            unsafe {
                self.ptr
                    .as_ptr()
                    .copy_to(self.ptr.as_ptr().add(self.head - bottom), bottom);

                self.ptr
                    .as_ptr()
                    .add(self.head)
                    .copy_to(self.ptr.as_ptr().add(self.head - self.len), top);
            }
            self.head -= self.len;
            return unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), self.len) };
        }

        // Copy the slices next to eachother and then swap them.
        if top < bottom {
            unsafe {
                self.ptr
                    .as_ptr()
                    .add(self.head)
                    .copy_to(self.ptr.as_ptr().add(bottom), top);
            }
            self.head = 0;
        } else {
            unsafe {
                self.ptr.as_ptr().copy_to(
                    self.ptr.as_ptr().add(self.capacity - top - bottom),
                    self.capacity - self.len,
                );
            }
            self.head = self.capacity - self.len;
        }
        let slice = unsafe { from_raw_parts_mut(self.ptr.as_ptr().add(self.head), self.len) };
        slice.rotate_left(bottom);
        slice
    }

    /*
     * Iteration.
     */

    pub fn iter(&self) -> Iter<T> {
        Iter { rb: self, index: 0 }
    }

    /*
     * Private methods.
     */

    /// Return true if the buffer is full and needs to be grown before elements can be added.
    fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// Grow the buffer by doubling its capacity.
    fn grow(&mut self) {
        let new_cap = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };

        // Safe unwrap because we know that `new_cap` is <= usize::MAX
        let new_layout = Layout::array::<T>(new_cap).unwrap();

        assert!(
            new_layout.size() <= isize::MAX as usize,
            "capacity overflow"
        );

        let new_ptr = if self.capacity == 0 {
            // Allocate a new buffer
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { std::alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // Abort if the allocation fails
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(ptr) => ptr,
            None => std::alloc::handle_alloc_error(new_layout),
        };

        // If the buffer wrapped around the end of the allocated memory, we need to update it now
        // that we have re-allocated.
        if self.head != 0 {
            if (self.capacity - self.head) >= self.head {
                // If the portion from the start of the buffer is smaller than the rest then we
                // move it to the end.
                //
                //        H                 C
                // [o, o, o, o, o, o, o, o]
                //
                //           H                        H+L
                // -> [., ., o, o, o, o, o, o, o, o., ., ., ., ., ., .]
                unsafe {
                    self.ptr
                        .as_ptr()
                        .copy_to(self.ptr.as_ptr().add(self.capacity), self.head);
                }
            } else {
                // If the portion from the start of the buffer is smaller than the rest then we
                // move it to the end.
                //
                //                    H     C
                // [o, o, o, o, o, o, o, o]
                //
                //                      H+L                      H    C
                // -> [o, o, o, o, o, o, ., ., ., ., ., ., ., ., o, o]
                unsafe {
                    self.ptr.as_ptr().add(self.head).copy_to(
                        self.ptr.as_ptr().add(new_cap - self.head),
                        self.capacity - self.head,
                    );
                }
                self.head = new_cap - self.head;
            }
        }
        self.capacity = new_cap;
    }
}

impl<T> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // Drop all elements in the buffer.
            while self.pop_front().is_some() {}
            // Deallocate the buffer.
            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe {
                std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    macro_rules! buffer_from_layout {
        ($len:tt: [$( $n:tt ),* ... $( $m:tt ),*]) => {{
            let mut rb = RingBuffer::<i32>::with_capacity($len);

            $(
                rb.push_back($n);
            )*

            let mut front = vec![$($m),*];
            front.reverse();
            for i in front {
                rb.push_front(i);
            }

            rb
        }};
    }

    #[test]
    fn test_new() {
        let rb = RingBuffer::<i32>::new();
        assert_eq!(rb.capacity, 0);
        assert_eq!(rb.head, 0);
        assert_eq!(rb.len, 0);
    }

    #[test]
    fn test_with_capactiy() {
        let rb = RingBuffer::<i32>::with_capacity(11);
        assert!(rb.capacity >= 11);
        assert_eq!(rb.head, 0);
        assert_eq!(rb.len, 0);
    }

    #[test]
    fn test_grow() {
        let mut rb = RingBuffer::<i32>::new();
        rb.grow();
        assert_eq!(rb.capacity, 1);
        rb.grow();
        assert_eq!(rb.capacity, 2);
        rb.grow();
        assert_eq!(rb.capacity, 4);
        assert!(!rb.is_full());
        assert!(rb.is_empty());
    }

    #[test]
    fn test_push_back() {
        let mut rb = RingBuffer::<i32>::new();

        rb.push_back(1);
        assert_eq!(rb.len, 1);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert!(rb.get(1).is_none());

        rb.push_back(2);
        assert_eq!(rb.len, 2);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert_eq!(rb[1], 2);

        rb.push_back(3);
        assert_eq!(rb.len, 3);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert_eq!(rb[1], 2);
        assert_eq!(rb[2], 3);
    }

    #[test]
    fn test_push_front() {
        let mut rb = RingBuffer::<i32>::new();

        rb.push_front(1);
        assert_eq!(rb.len, 1);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert!(rb.get(1).is_none());

        rb.push_front(2);
        assert_eq!(rb.len, 2);
        assert_eq!(rb.head, 1);
        assert_eq!(rb[0], 2);
        assert_eq!(rb[1], 1);

        //     H         H               H  (see `grow` for why this happens)
        // [1, 2] -> [1, 2, ., .] -> [., 2, 1, .] -> [3, 2, 1, .]
        rb.push_front(3);
        assert_eq!(rb.len, 3);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 3);
        assert_eq!(rb[1], 2);
        assert_eq!(rb[2], 1);
    }

    #[test]
    fn test_push() {
        let mut rb = RingBuffer::<i32>::with_capacity(10);

        //  H
        // [0, 1, 2, 3, 4, ., ., ., ., .]
        for i in 0..5 {
            rb.push_back(i);
        }

        //                          H
        // [0, 1, 2, 3, 4, ., ., ., 7, 6]
        rb.push_front(6);
        rb.push_front(7);
        assert_eq!(rb.len, 7);
        assert_eq!(rb.head, 8); // 10 - 2 = 8
        assert_eq!(rb[0], 7);
        assert_eq!(rb[6], 4);
    }

    #[test]
    fn test_pop_front() {
        let mut rb = RingBuffer::<i32>::with_capacity(10);

        // [3, ., ., 1, 2]
        rb.push_back(3);
        rb.push_front(2);
        rb.push_front(1);

        assert_eq!(rb.pop_front(), Some(1));
        assert_eq!(rb.pop_front(), Some(2));
        assert_eq!(rb.pop_front(), Some(3));
        assert_eq!(rb.pop_front(), None);
    }

    #[test]
    fn test_pop_back() {
        let mut rb = RingBuffer::<i32>::with_capacity(10);

        // [3, ., ., 1, 2]
        rb.push_back(3);
        rb.push_front(2);
        rb.push_front(1);

        assert_eq!(rb.pop_back(), Some(3));
        assert_eq!(rb.pop_back(), Some(2));
        assert_eq!(rb.pop_back(), Some(1));
        assert_eq!(rb.pop_back(), None);
    }

    #[test]
    fn test_make_contiguous() {
        let mut rb = RingBuffer::<i32>::with_capacity(10);

        // [3, 4, 5, ., ., ., ., ., 2, 1]
        rb.push_back(3);
        rb.push_back(4);
        rb.push_back(5);
        rb.push_front(2);
        rb.push_front(1);

        let slice = rb.make_contiguous();
        assert_eq!(slice.len(), 5);
        assert_eq!(slice, [1, 2, 3, 4, 5]);

        let mut rb = RingBuffer::<i32>::with_capacity(7);

        // [2, 3, 4, 5, ., ., 1]
        rb.push_back(2);
        rb.push_back(3);
        rb.push_back(4);
        rb.push_back(5);
        rb.push_front(1);

        let slice = rb.make_contiguous();
        assert_eq!(slice.len(), 5);
        assert_eq!(slice, [1, 2, 3, 4, 5]);

        let mut rb = RingBuffer::<i32>::with_capacity(7);

        // [3, 4, 5, 6, ., 1, 2]
        rb.push_back(3);
        rb.push_back(4);
        rb.push_back(5);
        rb.push_back(6);
        rb.push_front(2);
        rb.push_front(1);

        let slice = rb.make_contiguous();
        assert_eq!(slice.len(), 6);
        assert_eq!(slice, [1, 2, 3, 4, 5, 6]);

        let mut rb = RingBuffer::<i32>::with_capacity(6);

        // [5, 6, ., 1, 2, 3, 4]
        rb.push_back(5);
        rb.push_back(6);
        rb.push_front(4);
        rb.push_front(3);
        rb.push_front(2);
        rb.push_front(1);

        let slice = rb.make_contiguous();
        assert_eq!(slice.len(), 6);
        assert_eq!(slice, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_as_slices() {
        let mut rb = RingBuffer::<i32>::with_capacity(5);

        // [3, ., ., 1, 2]
        rb.push_back(3);
        rb.push_front(2);
        rb.push_front(1);

        let (first, second) = rb.as_slices();
        assert_eq!(first, &[1, 2]);
        assert_eq!(second, &[3]);

        rb.make_contiguous();
        let (first, second) = rb.as_slices();
        assert_eq!(first, &[1, 2, 3]);
        assert!(second.is_empty());
    }
}
